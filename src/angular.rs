use serde::Deserialize;
use std::path::PathBuf;
use std::{env, fs, vec};
use zed::lsp::{Completion, CompletionKind};
use zed::settings::LspSettings;
use zed::CodeLabelSpan;
use zed_extension_api::{self as zed, serde_json, Result};

// The Latest version of typescript isn't always compatible with angular see: https://angular.dev/reference/versions#unsupported-angular-versions
const DEFAULT_ANGULAR_LANGUAGE_SERVER_VERSION: &str = "19.0.4";
const DEFAULT_TYPESCRIPT_VERSION: &str = "5.7.3";

const SERVER_PATH: &str = "node_modules/@angular/language-server/index.js";
const TYPESCRIPT_TSDK_PATH: &str = "node_modules/typescript/lib";

const ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME: &str = "@angular/language-server";
const TYPESCRIPT_PACKAGE_NAME: &str = "typescript";

#[derive(Deserialize, Default)]
struct UserSettings {
    angular_language_server_version: Option<String>,
    typescript_version: Option<String>,
}

#[derive(Deserialize)]
struct PackageJson {
    version: String,
}

struct AngularExtension {
    did_find_server: bool,
    angular_language_server_version: String,
    typescript_version: String,
}

impl AngularExtension {
    #[allow(dead_code)]
    pub const LANGUAGE_SERVER_ID: &'static str = "angular";

    fn read_user_settings(
        &self,
        language_server_name: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<UserSettings> {
        let lsp_settings = LspSettings::for_worktree(&language_server_name.to_string(), worktree)?;

        if let Some(options) = lsp_settings.initialization_options {
            let user_settings: UserSettings = serde_json::from_value(options)
                .map_err(|e| format!("Failed to parse initialization_options: {}", e))?;
            Ok(user_settings)
        } else {
            Ok(UserSettings::default())
        }
    }
    fn file_exists_at_path(&self, path: &str) -> bool {
        fs::metadata(path).map_or(false, |stat| stat.is_file())
    }

    fn server_script_path(&mut self, language_server_id: &zed::LanguageServerId) -> Result<String> {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        self.ensure_packages_installed(language_server_id)?;

        if !self.file_exists_at_path(&SERVER_PATH) {
            return Err(format!(
                "Installed package '{}' did not contain expected path '{}'",
                ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME, SERVER_PATH
            )
            .into());
        }

        self.did_find_server = true;
        Ok(SERVER_PATH.to_string())
    }

    fn get_installed_version(&self, package_name: &str) -> Option<String> {
        let current_dir = Self::get_current_dir().ok()?;
        let package_json_path = current_dir
            .join("node_modules")
            .join(package_name)
            .join("package.json");

        let file = fs::File::open(package_json_path).ok()?;
        let package_json: PackageJson = serde_json::from_reader(file).ok()?;
        Some(package_json.version)
    }

    fn ensure_packages_installed(
        &mut self,
        language_server_id: &zed::LanguageServerId,
    ) -> Result<()> {
        let als_version = if self.angular_language_server_version == "latest" {
            zed::npm_package_latest_version(ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME)?
        } else {
            self.angular_language_server_version.clone()
        };

        let ts_version = if self.typescript_version == "latest" {
            zed::npm_package_latest_version(TYPESCRIPT_PACKAGE_NAME)?
        } else {
            self.typescript_version.clone()
        };

        let installed_als_version =
            self.get_installed_version(ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME);
        let installed_ts_version = self.get_installed_version(TYPESCRIPT_PACKAGE_NAME);

        if installed_als_version.as_deref() != Some(als_version.as_str())
            || !self.file_exists_at_path(SERVER_PATH)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            println!(
                "Installing {}@{}",
                ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME, als_version
            );
            zed::npm_install_package(ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME, &als_version).map_err(
                |error| {
                    format!(
                        "Failed to install package '{}': {}",
                        ANGULAR_LANGUAGE_SERVER_PACKAGE_NAME, error
                    )
                },
            )?;
        }

        if installed_ts_version.as_deref() != Some(ts_version.as_str()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            println!("Installing {}@{}", TYPESCRIPT_PACKAGE_NAME, ts_version);
            zed::npm_install_package(TYPESCRIPT_PACKAGE_NAME, &ts_version).map_err(|error| {
                format!(
                    "Failed to install package '{}': {}",
                    TYPESCRIPT_PACKAGE_NAME, error
                )
            })?;
        }

        Ok(())
    }

    fn get_current_dir() -> Result<PathBuf> {
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))
    }

    fn get_ng_probe_locations(worktree: Option<&zed::Worktree>) -> Vec<String> {
        let mut paths = vec![];

        if let Ok(path) = Self::get_current_dir() {
            paths.push(path.to_string_lossy().to_string());
        }

        if let Some(worktree) = worktree {
            paths.push(worktree.root_path());
        }

        paths
    }

    fn get_ts_probe_locations(worktree: Option<&zed::Worktree>) -> Vec<String> {
        let mut paths = vec![];

        if let Ok(path) = Self::get_current_dir() {
            paths.push(path.to_string_lossy().to_string());
        }

        if let Some(worktree) = worktree {
            paths.push(worktree.root_path());
        }

        paths
    }
}

impl zed::Extension for AngularExtension {
    fn new() -> Self {
        Self {
            did_find_server: false,
            angular_language_server_version: DEFAULT_ANGULAR_LANGUAGE_SERVER_VERSION.to_owned(),
            typescript_version: DEFAULT_TYPESCRIPT_VERSION.to_owned(),
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let user_settings = self.read_user_settings(language_server_id, worktree)?;

        if let Some(version) = user_settings.angular_language_server_version {
            self.angular_language_server_version = version;
        }

        if let Some(version) = user_settings.typescript_version {
            self.typescript_version = version;
        }

        let server_path = self.server_script_path(language_server_id)?;
        let current_dir = env::current_dir().unwrap_or(PathBuf::new());
        let full_path_to_server = current_dir.join(&server_path);

        let mut args = vec![full_path_to_server.to_string_lossy().to_string()];
        args.push("--stdio".to_string());

        args.push("--tsProbeLocations".to_string());
        args.extend(Self::get_ts_probe_locations(Some(worktree)));

        args.push("--ngProbeLocations".to_string());
        args.extend(Self::get_ng_probe_locations(Some(worktree)));

        args.push("--tsdk".to_string());
        args.push(TYPESCRIPT_TSDK_PATH.to_string());

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args,
            env: Default::default(),
        })
    }

    fn label_for_completion(
        &self,
        _language_server_id: &zed::LanguageServerId,
        completion: Completion,
    ) -> Option<zed::CodeLabel> {
        println!("Label for completion {:?}", completion.kind);
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
