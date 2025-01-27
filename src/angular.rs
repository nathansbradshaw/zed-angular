use serde::Deserialize;
use std::path::PathBuf;
use std::{env, fs, vec};
use zed::lsp::{Completion, CompletionKind};
use zed::settings::LspSettings;
use zed::CodeLabelSpan;
use zed_extension_api::{self as zed, serde_json, Result};

// The Latest version of typescript isn't always compatible with angular see: https://angular.dev/reference/versions#unsupported-angular-versions
const DEFAULT_ANGULAR_LANGUAGE_SERVICE_VERSION: &str = "18.2.0";
const DEFAULT_TYPESCRIPT_VERSION: &str = "5.5.4";

const SERVER_PATH: &str = "node_modules/@angular/language-server/index.js";
const TYPESCRIPT_TSDK_PATH: &str = "node_modules/typescript/lib";

const ANGULAR_SERVER_PACKAGE_NAME: &str = "@angular/language-server";
const TYPESCRIPT_PACKAGE_NAME: &str = "typescript";

#[derive(Deserialize, Default)]
struct UserSettings {
    angular_language_service_version: Option<String>,
    typescript_version: Option<String>,
}

struct AngularExtension {
    did_find_server: bool,
    angular_language_service_version: String,
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
        let server_exists = self.file_exists_at_path(&SERVER_PATH);

        if self.did_find_server && server_exists {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::CheckingForUpdate,
            );

            // TODO only install new version if there are change
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        self.install_packages()?;

        if !self.file_exists_at_path(&SERVER_PATH) {
            return Err(format!(
                "Installed package '{}' did not contain expected path '{}'",
                ANGULAR_SERVER_PACKAGE_NAME, SERVER_PATH
            )
            .into());
        }

        self.did_find_server = true;
        Ok(SERVER_PATH.to_string())
    }

    fn install_packages(&mut self) -> Result<()> {
        let als_version = if self.angular_language_service_version == "latest" {
            zed::npm_package_latest_version(ANGULAR_SERVER_PACKAGE_NAME)?
        } else {
            self.angular_language_service_version.clone()
        };

        let ts_version = if self.typescript_version == "latest" {
            zed::npm_package_latest_version(TYPESCRIPT_PACKAGE_NAME)?
        } else {
            self.typescript_version.clone()
        };

        println!(
            "Installing {}@{}, {}@{}",
            ANGULAR_SERVER_PACKAGE_NAME, als_version, TYPESCRIPT_PACKAGE_NAME, ts_version
        );

        zed::npm_install_package(ANGULAR_SERVER_PACKAGE_NAME, &als_version).map_err(|error| {
            format!(
                "Failed to install package '{}': {}",
                ANGULAR_SERVER_PACKAGE_NAME, error
            )
        })?;
        zed::npm_install_package(TYPESCRIPT_PACKAGE_NAME, &ts_version).map_err(|error| {
            format!(
                "Failed to install package '{}': {}",
                ANGULAR_SERVER_PACKAGE_NAME, error
            )
        })?;

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
            angular_language_service_version: DEFAULT_ANGULAR_LANGUAGE_SERVICE_VERSION.to_owned(),
            typescript_version: DEFAULT_TYPESCRIPT_VERSION.to_owned(),
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let user_settings = self.read_user_settings(language_server_id, worktree)?;

        if let Some(version) = user_settings.angular_language_service_version {
            self.angular_language_service_version = version;
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
