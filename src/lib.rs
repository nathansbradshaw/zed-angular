use std::path::PathBuf;
use std::{env, fs};
use zed::lsp::{Completion, CompletionKind};
use zed::CodeLabelSpan;
use zed_extension_api::{self as zed, serde_json, Result};

struct AngularExtension {
    did_find_server: bool,
}

const SERVER_PATH: &str = "node_modules/@angular/language-server/index.js";
const PACKAGE_NAME: &str = "@angular/language-server";
const TYPESCRIPT_PACKAGE: &str = "typescript";

impl AngularExtension {
    fn server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).map_or(false, |stat| stat.is_file())
    }
    fn server_script_path(&mut self, id: &zed::LanguageServerId) -> Result<String> {
        println!("SERVER SCRIPT PATH");
        let server_exists = self.server_exists();
        if self.did_find_server && server_exists {
            return Ok(SERVER_PATH.to_string());
        }

        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;
        let ts_version = zed::npm_package_latest_version(TYPESCRIPT_PACKAGE)?;

        // Log the fetched versions
        println!("Latest version for {}: {}", PACKAGE_NAME, version);
        println!("Latest version for {}: {}", TYPESCRIPT_PACKAGE, ts_version);

        // Install TypeScript if necessary
        if zed::npm_package_installed_version(TYPESCRIPT_PACKAGE)?.as_ref() != Some(&ts_version) {
            zed::set_language_server_installation_status(
                id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            zed::npm_install_package(TYPESCRIPT_PACKAGE, &ts_version)?;
            println!("TypeScript installed or updated successfully.");
        }

        // Install Angular Language Server if necessary
        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            let result = zed::npm_install_package(PACKAGE_NAME, &version);
            if let Err(error) = result {
                println!("Error installing {}: {}", PACKAGE_NAME, error);
                return Err(error.into());
            }
            println!("{} installed or updated successfully.", PACKAGE_NAME);
        }

        self.did_find_server = true;
        Ok(SERVER_PATH.to_string())
    }
}
impl zed::Extension for AngularExtension {
    fn new() -> Self {
        println!("NEW!");
        Self {
            did_find_server: false,
        }
    }

    fn language_server_command(
        &mut self,
        id: &zed::LanguageServerId,
        _: &zed::Worktree,
    ) -> Result<zed::Command> {
        println!("LANGUAGE SERVER COMMANDS");
        let server_path = self.server_script_path(id)?;
        let current_dir = env::current_dir().unwrap_or(PathBuf::new());
        let full_path_to_server = current_dir.join(&server_path);
        let node_modules_path = current_dir.join("node_modules");
        let ts_lib_path = node_modules_path
            .join("typescript/lib")
            .to_string_lossy()
            .to_string();
        let ng_service_path = node_modules_path
            .join("@angular/language-service/bin")
            .to_string_lossy()
            .to_string();
        println!(
            "command: {:?} --stdio --tsProbeLocations {:?} --ngProbeLocations {:?} ",
            full_path_to_server, ts_lib_path, ng_service_path
        );

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![
                full_path_to_server.to_string_lossy().to_string(),
                "--stdio".to_string(),
                "--tsProbeLocations".to_string(),
                ts_lib_path,
                "--ngProbeLocations".to_string(),
                ng_service_path,
            ],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _: &zed::LanguageServerId,
        _: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let current_dir = env::current_dir().unwrap_or(PathBuf::new());
        let node_modules_path = current_dir.join("node_modules");
        let ts_lib_path = node_modules_path
            .join("typescript/lib")
            .to_string_lossy()
            .to_string();
        let ng_service_path = node_modules_path
            .join("@angular/language-service/bin")
            .to_string_lossy()
            .to_string();
        Ok(Some(serde_json::json!({
            "typescript": {
                "tsdk": ts_lib_path,
            },
            "tsProbeLocations": ts_lib_path,
            "ngProbeLocations": ng_service_path,
        })))
    }

    fn label_for_completion(
        &self,
        _language_server_id: &zed::LanguageServerId,
        completion: Completion,
    ) -> Option<zed::CodeLabel> {
        println!("Label for completion {:?}", completion.kind);
        let highlight_name = match completion.kind? {
            CompletionKind::Class | CompletionKind::Interface => "type",
            CompletionKind::Constructor => "type",
            CompletionKind::Constant => "constant",
            CompletionKind::Function | CompletionKind::Method => "function",
            CompletionKind::Property | CompletionKind::Field => "property",
            CompletionKind::Variable => "variable",
            CompletionKind::Keyword => "keyword",
            CompletionKind::Value => "value",
            _ => return None,
        };

        let len = completion.label.len();
        let name_span = CodeLabelSpan::literal(completion.label, Some(highlight_name.to_string()));

        Some(zed::CodeLabel {
            code: Default::default(),
            spans: if let Some(detail) = completion.detail {
                vec![
                    name_span,
                    CodeLabelSpan::literal(" ", None),
                    CodeLabelSpan::literal(detail, None),
                ]
            } else {
                vec![name_span]
            },
            filter_range: (0..len).into(),
        })
    }
}

zed::register_extension!(AngularExtension);
