use serde::Deserialize;
use std::collections::HashMap;
use std::{env, fs, vec};
use zed::lsp::{Completion, CompletionKind};
use zed::settings::LspSettings;
use zed::CodeLabelSpan;
use zed_extension_api::{self as zed, serde_json, Result};

// The latest version of TypeScript isn't always compatible with Angular, see:
// https://angular.dev/reference/versions#unsupported-angular-versions
// These defaults are only used as a last-resort fallback when the project's
// Angular version cannot be detected.
const DEFAULT_ANGULAR_LANGUAGE_SERVER_VERSION: &str = "21.2.17";
const DEFAULT_TYPESCRIPT_VERSION: &str = "5.9.3";

// Path to the language server installed in the extension's own sandbox
// (relative to the extension's working directory).
const SERVER_PATH: &str = "node_modules/@angular/language-server/index.js";
const TYPESCRIPT_TSDK_PATH: &str = "node_modules/typescript/lib";

// Paths inside the *project's* worktree, used to detect a locally installed
// language server and TypeScript SDK. `read_text_file` is the only filesystem
// probe available against the worktree, so we read tiny marker files to check
// for existence.
const PROJECT_SERVER_REL_PATH: &str = "node_modules/@angular/language-server/index.js";
const PROJECT_TSDK_REL_DIR: &str = "node_modules/typescript/lib";
const PROJECT_TSDK_PROBE_FILE: &str = "node_modules/typescript/package.json";

const ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME: &str = "@angular/language-server";
const TYPESCRIPT_PACKAGE_NAME: &str = "typescript";

const ANGULAR_CORE_PACKAGE_NAME: &str = "@angular/core";
const ANGULAR_LANGUAGE_SERVICE_PACKAGE_NAME: &str = "@angular/language-service";

/// Maps an Angular major version to a concrete, known-good
/// (`@angular/language-server`, `typescript`) version pair.
///
/// The language-server patch is the latest published for that major; the
/// TypeScript version is chosen comfortably inside Angular's supported range
/// to avoid the "unsupported TypeScript version" warning. See:
/// https://angular.dev/reference/versions
///
/// Majors below the lowest entry fall back to the lowest entry; majors above
/// the highest known entry fall back to the modern defaults.
const ANGULAR_VERSION_TABLE: &[(u32, &str, &str)] = &[
    (13, "13.3.4", "4.6.4"),
    (14, "14.2.0", "4.8.4"),
    (15, "15.2.1", "4.9.5"),
    (16, "16.2.0", "5.1.6"),
    (17, "17.3.2", "5.4.5"),
    (18, "18.2.0", "5.5.4"),
    (19, "19.2.4", "5.8.3"),
    (20, "20.3.0", "5.8.3"),
    (21, "21.2.17", "5.9.3"),
];

/// User settings read from the language server's `initialization_options` in
/// Zed's `settings.json`. Version keys use snake_case; the Angular feature keys
/// mirror the dot-namespaced keys of the official VSCode extension verbatim, so
/// configuration can be copied straight from Angular's docs.
#[derive(Deserialize, Default)]
struct UserSettings {
    // Version management (existing behaviour).
    angular_language_server_version: Option<String>,
    typescript_version: Option<String>,

    // Completion behaviour (forwarded as CLI flags, matching the VSCode client).
    #[serde(rename = "angular.suggest.includeAutomaticOptionalChainCompletions")]
    include_automatic_optional_chain_completions: Option<bool>,
    #[serde(rename = "angular.suggest.includeCompletionsWithSnippetText")]
    include_completions_with_snippet_text: Option<bool>,
    /// Auto-imports. Defaults to `true` (same as VSCode) when unset.
    #[serde(rename = "angular.suggest.autoImports")]
    auto_imports: Option<bool>,

    // Diagnostics / strictness.
    #[serde(rename = "angular.forceStrictTemplates")]
    force_strict_templates: Option<bool>,
    /// Comma-separated list of diagnostic codes to suppress, e.g. "-992008,-998001".
    #[serde(rename = "angular.suppressAngularDiagnosticCodes")]
    suppress_angular_diagnostic_codes: Option<String>,

    // File watching.
    #[serde(rename = "angular.server.useClientSideFileWatcher")]
    use_client_side_file_watcher: Option<bool>,

    /// Catch-all for any remaining keys (notably the `angular.inlayHints.*`
    /// family) so they can be forwarded verbatim through workspace configuration
    /// without enumerating every one. The known keys above are captured into
    /// their own fields and won't appear here.
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

/// Minimal view of a project's `package.json` needed to detect its Angular version.
#[derive(Deserialize, Default)]
struct PackageJson {
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
}

/// How the Angular Language Server should be launched for a given worktree.
enum ServerSource {
    /// Use the language server already installed in the project's `node_modules`.
    /// Both paths are absolute. No npm install is performed; this guarantees an
    /// exact version match with the project and covers monorepos automatically.
    ProjectNodeModules {
        index_js_abs: String,
        tsdk_abs: String,
    },
    /// Install (if needed) and use the language server in the extension's sandbox
    /// at the resolved versions.
    Managed {
        als_version: String,
        ts_version: String,
    },
}

/// Version resolution is fully per-worktree and stateless, so the extension
/// holds no fields. A single instance may serve multiple worktrees.
struct AngularExtension;

/// A version parsed from an npm range spec.
///
/// Handles common forms: `"^17.3.0"`, `"~18.0.0-rc.1"`, `">=19.0.0 <20"`,
/// `"17.3.0"`, `"v16.1.0"`, `"18.x"`. Returns `None` for non-numeric specs
/// such as `"latest"`, `"next"`, `"*"`, `"workspace:*"`, or git/URL specs, so
/// the caller can fall back to a sensible default.
struct ParsedVersion {
    /// The major version number (e.g. 17).
    major: u32,
    /// The concrete version token with range operators stripped (e.g. "17.3.0"),
    /// taken up to the first whitespace so `">=17.0.0 <18"` yields `"17.0.0"`.
    cleaned: String,
}

fn parse_version(range: &str) -> Option<ParsedVersion> {
    let trimmed = range.trim();
    // Find the first digit; everything before it is a range operator/prefix
    // (`^`, `~`, `>=`, `>`, `=`, `v`, whitespace, etc.).
    let start = trimmed.find(|c: char| c.is_ascii_digit())?;
    // Reject specs where the prefix is clearly not a simple range operator
    // (e.g. "workspace:1.0.0", "npm:foo@1.2.3").
    let prefix = &trimmed[..start];
    if prefix.contains(':') || prefix.contains('@') {
        return None;
    }

    let cleaned: String = trimmed[start..]
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();

    let major = cleaned
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .ok()?;

    Some(ParsedVersion { major, cleaned })
}

/// Inserts `value` into `root` at a dot-delimited `path`, creating intermediate
/// objects as needed. Used to merge the user's flat VSCode-style config keys
/// (e.g. `"angular.inlayHints.variableTypes.ifAliasTypes"`) into the nested
/// structure the language server expects. Non-object intermediates are
/// overwritten with objects.
fn set_nested(root: &mut serde_json::Value, path: &str, value: serde_json::Value) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = root;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Some(obj) = current.as_object_mut() {
                obj.insert((*part).to_string(), value);
            }
            return;
        }
        // Descend, creating an object if the slot is missing or not an object.
        let obj = match current.as_object_mut() {
            Some(obj) => obj,
            None => return,
        };
        let entry = obj
            .entry((*part).to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if !entry.is_object() {
            *entry = serde_json::Value::Object(serde_json::Map::new());
        }
        current = entry;
    }
}

/// Maps an Angular major version to concrete (`@angular/language-server`,
/// `typescript`) versions. Returns `None` for majors above the known table,
/// signalling the caller to use the modern defaults.
fn map_angular_major_to_versions(major: u32) -> Option<(String, String)> {
    let lowest = ANGULAR_VERSION_TABLE.first().expect("table is non-empty");
    let highest = ANGULAR_VERSION_TABLE.last().expect("table is non-empty");

    // Below the lowest known major: clamp to the lowest entry.
    if major < lowest.0 {
        return Some((lowest.1.to_string(), lowest.2.to_string()));
    }
    // Above the highest known major: defer to the modern defaults.
    if major > highest.0 {
        return None;
    }

    ANGULAR_VERSION_TABLE
        .iter()
        .find(|(m, _, _)| *m == major)
        .map(|(_, als, ts)| (als.to_string(), ts.to_string()))
}

impl AngularExtension {
    #[allow(dead_code)]
    pub const LANGUAGE_SERVER_ID: &'static str = "angular";

    fn read_user_settings(
        &self,
        language_server_name: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<UserSettings> {
        let lsp_settings = LspSettings::for_worktree(language_server_name.as_ref(), worktree)?;

        if let Some(options) = lsp_settings.initialization_options {
            let user_settings: UserSettings = serde_json::from_value(options)
                .map_err(|e| format!("Failed to parse initialization_options: {}", e))?;
            Ok(user_settings)
        } else {
            Ok(UserSettings::default())
        }
    }

    fn file_exists_at_path(&self, path: &str) -> bool {
        fs::metadata(path).is_ok_and(|stat| stat.is_file())
    }

    /// Reads the project's `package.json` once and parses the Angular version
    /// from `@angular/core` (preferred) or `@angular/language-service`.
    /// Returns `None` if there is no `package.json` or no usable version.
    fn detect_angular_version(worktree: &zed::Worktree) -> Option<ParsedVersion> {
        let contents = worktree.read_text_file("package.json").ok()?;
        let pkg: PackageJson = serde_json::from_str(&contents).ok()?;

        let lookup = |name: &str| -> Option<&String> {
            pkg.dependencies
                .get(name)
                .or_else(|| pkg.dev_dependencies.get(name))
        };

        let range = lookup(ANGULAR_CORE_PACKAGE_NAME)
            .or_else(|| lookup(ANGULAR_LANGUAGE_SERVICE_PACKAGE_NAME))?;

        parse_version(range)
    }

    /// Builds the feature CLI flags forwarded to the language server, mirroring
    /// the official VSCode client's `constructArgs`. `core_version` (when known)
    /// is passed as `--angularCoreVersion` so the compiler stays compatible with
    /// older Angular versions.
    fn build_feature_args(settings: &UserSettings, core_version: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();

        if settings
            .include_automatic_optional_chain_completions
            .unwrap_or(false)
        {
            args.push("--includeAutomaticOptionalChainCompletions".to_string());
        }

        if settings
            .include_completions_with_snippet_text
            .unwrap_or(false)
        {
            args.push("--includeCompletionsWithSnippetText".to_string());
        }

        // Auto-imports default to true, matching the VSCode client.
        args.push("--includeCompletionsForModuleExports".to_string());
        args.push(settings.auto_imports.unwrap_or(true).to_string());

        if settings.force_strict_templates.unwrap_or(false) {
            args.push("--forceStrictTemplates".to_string());
        }

        if let Some(codes) = &settings.suppress_angular_diagnostic_codes {
            if !codes.trim().is_empty() {
                args.push("--suppressAngularDiagnosticCodes".to_string());
                args.push(codes.clone());
            }
        }

        if settings.use_client_side_file_watcher.unwrap_or(false) {
            args.push("--useClientSideFileWatcher".to_string());
        }

        // Pass the project's Angular core version so the compiler targets it for
        // maximum compatibility (e.g. it won't import v21 APIs into a v13 project).
        if let Some(core_version) = core_version {
            args.push("--angularCoreVersion".to_string());
            args.push(core_version.to_string());
        }

        args
    }

    /// Builds the nested workspace configuration returned to the server.
    ///
    /// The Angular Language Server requests sections such as `angular.inlayHints`
    /// via `workspace/configuration` and then flattens the *nested* object it
    /// receives (e.g. `{variableTypes: {forLoopVariableTypes: true}}` becomes
    /// `angular.inlayHints.variableTypes.forLoopVariableTypes`). So the config
    /// must be nested, not dot-flattened.
    ///
    /// A sensible set of hints is enabled by default; any matching key the user
    /// sets in `initialization_options` (flat, VSCode-style) overrides it.
    /// `editor.inlayHints.enabled` must be on for the server to emit hints.
    fn build_workspace_configuration(settings: &UserSettings) -> serde_json::Value {
        use serde_json::json;

        // Default-on inlay hints (a useful subset of the VSCode defaults),
        // expressed as the nested object the server expects under `angular`.
        let mut config = json!({
            "editor": { "inlayHints": { "enabled": "on" } },
            "angular": {
                "inlayHints": {
                    "variableTypes": {
                        "forLoopVariableTypes": true,
                        "ifAliasTypes": true,
                        "letDeclarationTypes": true,
                        "referenceVariableTypes": true
                    },
                    "bindingHints": {
                        "pipeOutputTypes": true,
                        "twoWayBindingSignalTypes": true
                    },
                    "eventHints": {
                        "parameterTypes": true
                    },
                    "controlFlowHints": {
                        "switchExpressionTypes": true
                    }
                }
            }
        });

        // Apply the user's flat, dot-namespaced overrides on top by walking the
        // dotted key into the nested structure (e.g.
        // "angular.inlayHints.variableTypes.ifAliasTypes" -> nested path).
        if let Some(force) = settings.force_strict_templates {
            set_nested(&mut config, "angular.forceStrictTemplates", json!(force));
        }
        for (key, value) in &settings.extra {
            set_nested(&mut config, key, value.clone());
        }

        config
    }

    /// Decides how to launch the language server for this worktree, in order of
    /// priority: manual override -> project's own language server -> version
    /// derived from `detected` (the parsed `package.json` version) -> defaults.
    fn resolve_server(
        &self,
        worktree: &zed::Worktree,
        settings: &UserSettings,
        detected: Option<&ParsedVersion>,
    ) -> ServerSource {
        // 1. Manual override via initialization_options always wins.
        if settings.angular_language_server_version.is_some()
            || settings.typescript_version.is_some()
        {
            let als = settings
                .angular_language_server_version
                .clone()
                .unwrap_or_else(|| DEFAULT_ANGULAR_LANGUAGE_SERVER_VERSION.to_string());
            let ts = settings
                .typescript_version
                .clone()
                .unwrap_or_else(|| DEFAULT_TYPESCRIPT_VERSION.to_string());
            return ServerSource::Managed {
                als_version: als,
                ts_version: ts,
            };
        }

        // 2. Prefer the language server already installed in the project, when
        //    both it and a local TypeScript SDK are present. Guarantees an exact
        //    match and covers monorepos (nx/pnpm hoisted node_modules).
        let has_project_server = worktree.read_text_file(PROJECT_SERVER_REL_PATH).is_ok();
        let has_project_tsdk = worktree.read_text_file(PROJECT_TSDK_PROBE_FILE).is_ok();
        if has_project_server && has_project_tsdk {
            let root = worktree.root_path();
            return ServerSource::ProjectNodeModules {
                index_js_abs: format!("{root}/{PROJECT_SERVER_REL_PATH}"),
                tsdk_abs: format!("{root}/{PROJECT_TSDK_REL_DIR}"),
            };
        }

        // 3. Derive a compatible version from the detected Angular version.
        if let Some((als, ts)) = detected.and_then(|v| map_angular_major_to_versions(v.major)) {
            return ServerSource::Managed {
                als_version: als,
                ts_version: ts,
            };
        }

        // 4. Fallback to modern defaults.
        ServerSource::Managed {
            als_version: DEFAULT_ANGULAR_LANGUAGE_SERVER_VERSION.to_string(),
            ts_version: DEFAULT_TYPESCRIPT_VERSION.to_string(),
        }
    }

    /// Ensures the language server is installed in the extension's sandbox at the
    /// requested versions and returns the (relative) path to its `index.js`.
    fn ensure_managed_server(
        &self,
        language_server_id: &zed::LanguageServerId,
        als_version: &str,
        ts_version: &str,
    ) -> Result<String> {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        self.install_packages(als_version, ts_version)?;

        if !self.file_exists_at_path(SERVER_PATH) {
            return Err(format!(
                "Installed package '{}' did not contain expected path '{}'",
                ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME, SERVER_PATH
            ));
        }

        Ok(SERVER_PATH.to_string())
    }

    fn install_packages(&self, als_version: &str, ts_version: &str) -> Result<()> {
        let als_version = if als_version == "latest" {
            zed::npm_package_latest_version(ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME)?
        } else {
            als_version.to_string()
        };

        let ts_version = if ts_version == "latest" {
            zed::npm_package_latest_version(TYPESCRIPT_PACKAGE_NAME)?
        } else {
            ts_version.to_string()
        };

        // Only (re)install when the currently installed version differs from the
        // target, so unchanged projects don't trigger a download on every start.
        let als_installed =
            zed::npm_package_installed_version(ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME)?;
        let ts_installed = zed::npm_package_installed_version(TYPESCRIPT_PACKAGE_NAME)?;

        let als_up_to_date = als_installed.as_deref() == Some(als_version.as_str());
        let ts_up_to_date = ts_installed.as_deref() == Some(ts_version.as_str());

        if als_up_to_date && ts_up_to_date && self.file_exists_at_path(SERVER_PATH) {
            println!(
                "[angular] {}@{} and {}@{} already installed; skipping",
                ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME,
                als_version,
                TYPESCRIPT_PACKAGE_NAME,
                ts_version
            );
            return Ok(());
        }

        println!(
            "[angular] Installing {}@{}, {}@{}",
            ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME, als_version, TYPESCRIPT_PACKAGE_NAME, ts_version
        );

        zed::npm_install_package(ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME, &als_version).map_err(
            |error| {
                format!(
                    "Failed to install package '{}': {}",
                    ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME, error
                )
            },
        )?;
        zed::npm_install_package(TYPESCRIPT_PACKAGE_NAME, &ts_version).map_err(|error| {
            format!(
                "Failed to install package '{}': {}",
                TYPESCRIPT_PACKAGE_NAME, error
            )
        })?;

        Ok(())
    }

    /// Locations the language server probes to find TypeScript and Angular: the
    /// extension's own working directory and the project root. Used for both
    /// `--tsProbeLocations` and `--ngProbeLocations`.
    fn probe_locations(worktree: &zed::Worktree) -> Vec<String> {
        let mut paths = vec![];
        if let Ok(dir) = env::current_dir() {
            paths.push(dir.to_string_lossy().to_string());
        }
        paths.push(worktree.root_path());
        paths
    }
}

impl zed::Extension for AngularExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let user_settings = self.read_user_settings(language_server_id, worktree)?;

        // Read the project's Angular version once; both server resolution and
        // the feature flags below reuse it.
        let detected = Self::detect_angular_version(worktree);

        // Resolve which language server to run for this project, then build the
        // launch command. `index_js` is an absolute path to the server entry;
        // `tsdk` is the TypeScript lib directory passed to `--tsdk`.
        let (index_js, tsdk) = match self.resolve_server(
            worktree,
            &user_settings,
            detected.as_ref(),
        ) {
            ServerSource::ProjectNodeModules {
                index_js_abs,
                tsdk_abs,
            } => {
                println!("[angular] Using language server from project node_modules");
                (index_js_abs, tsdk_abs)
            }
            ServerSource::Managed {
                als_version,
                ts_version,
            } => {
                println!(
                        "[angular] Using @angular/language-server@{als_version} with typescript@{ts_version}"
                    );
                let server_path =
                    self.ensure_managed_server(language_server_id, &als_version, &ts_version)?;
                let current_dir = env::current_dir().unwrap_or_default();
                let full_path_to_server = current_dir.join(&server_path);
                (
                    full_path_to_server.to_string_lossy().to_string(),
                    TYPESCRIPT_TSDK_PATH.to_string(),
                )
            }
        };

        let mut args = vec![index_js];
        args.push("--stdio".to_string());

        let probe_locations = Self::probe_locations(worktree);
        args.push("--tsProbeLocations".to_string());
        args.extend(probe_locations.iter().cloned());

        args.push("--ngProbeLocations".to_string());
        args.extend(probe_locations);

        args.push("--tsdk".to_string());
        args.push(tsdk);

        // Forward feature flags derived from the user's settings and the
        // detected Angular version (completions, strict templates, suppressed
        // diagnostics, --angularCoreVersion, …), mirroring the VSCode client.
        let core_version = detected.as_ref().map(|v| v.cleaned.as_str());
        args.extend(Self::build_feature_args(&user_settings, core_version));

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args,
            env: Default::default(),
        })
    }

    /// Responds to the server's `workspace/configuration` requests with the
    /// `angular` section. The Angular Language Server fetches inlay-hint
    /// settings dynamically through this channel, so returning them here is what
    /// makes inlay hints configurable (and on by default).
    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = self.read_user_settings(language_server_id, worktree)?;
        Ok(Some(Self::build_workspace_configuration(&settings)))
    }

    fn label_for_completion(
        &self,
        _language_server_id: &zed::LanguageServerId,
        completion: Completion,
    ) -> Option<zed::CodeLabel> {
        let highlight_name = match completion.kind? {
            CompletionKind::Class | CompletionKind::Interface => "type",
            CompletionKind::Constructor => "constructor",
            CompletionKind::Constant => "constant",
            CompletionKind::Function | CompletionKind::Method => "function",
            CompletionKind::Property | CompletionKind::Field => "property",
            CompletionKind::Variable => "variable",
            CompletionKind::Keyword => "keyword",
            CompletionKind::Enum => "enum",
            CompletionKind::Module => "module",
            _ => return None,
        };

        let len = completion.label.len();
        let name_span = CodeLabelSpan::literal(completion.label, Some(highlight_name.to_string()));

        let spans = if let Some(detail) = completion.detail {
            vec![
                name_span,
                CodeLabelSpan::literal(" ", None),
                CodeLabelSpan::literal(detail, Some("detail".to_string())),
            ]
        } else {
            vec![name_span]
        };

        Some(zed::CodeLabel {
            code: Default::default(),
            spans,
            filter_range: (0..len).into(),
        })
    }
}

zed::register_extension!(AngularExtension);

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: assert a range parses to the given (major, cleaned) pair.
    fn assert_parsed(range: &str, major: u32, cleaned: &str) {
        let parsed = parse_version(range).expect("should parse");
        assert_eq!(parsed.major, major, "major for {range:?}");
        assert_eq!(parsed.cleaned, cleaned, "cleaned for {range:?}");
    }

    #[test]
    fn parses_common_version_ranges() {
        assert_parsed("^17.3.0", 17, "17.3.0");
        assert_parsed("~18.0.0-rc.1", 18, "18.0.0-rc.1");
        assert_parsed(">=19.0.0 <20.0.0", 19, "19.0.0");
        assert_parsed("17.3.0", 17, "17.3.0");
        assert_parsed("v16.1.0", 16, "16.1.0");
        assert_parsed("18.x", 18, "18.x");
        assert_parsed("  ^20.0.0  ", 20, "20.0.0");
        assert_parsed("21", 21, "21");
    }

    #[test]
    fn rejects_non_numeric_ranges() {
        for range in [
            "latest",
            "next",
            "*",
            "workspace:*",
            "npm:@angular/core@17.0.0",
            "",
        ] {
            assert!(parse_version(range).is_none(), "{range:?} should not parse");
        }
    }

    #[test]
    fn maps_known_majors_to_versions() {
        assert_eq!(
            map_angular_major_to_versions(17),
            Some(("17.3.2".to_string(), "5.4.5".to_string()))
        );
        assert_eq!(
            map_angular_major_to_versions(13),
            Some(("13.3.4".to_string(), "4.6.4".to_string()))
        );
        assert_eq!(
            map_angular_major_to_versions(21),
            Some(("21.2.17".to_string(), "5.9.3".to_string()))
        );
    }

    #[test]
    fn clamps_below_lowest_and_defers_above_highest() {
        // Below the lowest known major clamps to the oldest entry (v13).
        assert_eq!(
            map_angular_major_to_versions(11),
            Some(("13.3.4".to_string(), "4.6.4".to_string()))
        );
        // Newer than the table -> defer to defaults.
        assert_eq!(map_angular_major_to_versions(99), None);
    }

    #[test]
    fn set_nested_creates_and_overrides_paths() {
        use serde_json::json;

        let mut root =
            json!({ "angular": { "inlayHints": { "variableTypes": { "ifAliasTypes": true } } } });

        // Override an existing leaf.
        set_nested(
            &mut root,
            "angular.inlayHints.variableTypes.ifAliasTypes",
            json!(false),
        );
        assert_eq!(
            root["angular"]["inlayHints"]["variableTypes"]["ifAliasTypes"],
            json!(false)
        );

        // Create a brand-new nested path.
        set_nested(
            &mut root,
            "angular.inlayHints.bindingHints.pipeOutputTypes",
            json!(true),
        );
        assert_eq!(
            root["angular"]["inlayHints"]["bindingHints"]["pipeOutputTypes"],
            json!(true)
        );
    }

    #[test]
    fn workspace_configuration_enables_hints_by_default() {
        let settings = UserSettings::default();
        let config = AngularExtension::build_workspace_configuration(&settings);
        assert_eq!(config["editor"]["inlayHints"]["enabled"], "on");
        assert_eq!(
            config["angular"]["inlayHints"]["variableTypes"]["forLoopVariableTypes"],
            true
        );
    }

    #[test]
    fn workspace_configuration_respects_user_override() {
        let mut settings = UserSettings::default();
        settings.extra.insert(
            "angular.inlayHints.variableTypes.forLoopVariableTypes".to_string(),
            serde_json::json!(false),
        );
        let config = AngularExtension::build_workspace_configuration(&settings);
        assert_eq!(
            config["angular"]["inlayHints"]["variableTypes"]["forLoopVariableTypes"],
            false
        );
    }
}
