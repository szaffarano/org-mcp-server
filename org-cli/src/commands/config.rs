use crate::config::CliAppConfig;
use anyhow::Result;
use clap::{Args, Subcommand};
use org_core::config::default_config_path;

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Initialize a default configuration file
    Init,
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
}

impl ConfigCommand {
    pub fn execute(&self, config_file: Option<String>) -> Result<()> {
        match &self.action {
            ConfigAction::Init => self.init_config(config_file),
            ConfigAction::Show => self.show_config(config_file),
            ConfigAction::Path => self.show_path(config_file),
        }
    }

    fn init_config(&self, config_file: Option<String>) -> Result<()> {
        let mut config_path = if let Some(path) = config_file {
            std::path::PathBuf::from(path)
        } else {
            default_config_path()?
        };

        if config_path.extension().is_none() {
            config_path.set_extension("toml");
        }

        if config_path.exists() {
            println!(
                "Configuration file already exists at: {}",
                config_path.display()
            );
            println!("Use 'org config show' to view current configuration");
            return Ok(());
        }

        let default_config_str = CliAppConfig::generate_default_config()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&config_path, default_config_str)?;

        println!(
            "Default configuration file created at: {}",
            config_path.display()
        );
        println!("Edit this file to customize your org-mode setup");

        Ok(())
    }

    fn show_config(&self, config_file: Option<String>) -> Result<()> {
        match CliAppConfig::load(config_file.clone(), None, None) {
            Ok(config) => {
                #[derive(serde::Serialize)]
                struct DisplayConfig<'a> {
                    org: &'a org_core::OrgConfig,
                    cli: &'a crate::config::CliConfig,
                    logging: &'a org_core::LoggingConfig,
                }

                let display = DisplayConfig {
                    org: &config.org,
                    cli: &config.cli,
                    logging: &config.logging,
                };

                let config_str = toml::to_string_pretty(&display)?;
                println!("{}", config_str);
            }
            Err(_) => {
                let config_path = if let Some(path) = config_file.as_ref() {
                    std::path::PathBuf::from(path)
                } else {
                    default_config_path()?
                };

                let mut actual_path = None;
                if config_path.exists() {
                    actual_path = Some(config_path.clone());
                } else if let Some(parent) = config_path.parent() {
                    for ext in &["toml", "yaml", "yml", "json"] {
                        let path_with_ext = parent.join(format!("config.{ext}"));
                        if path_with_ext.exists() {
                            actual_path = Some(path_with_ext);
                            break;
                        }
                    }
                }

                if let Some(path) = actual_path {
                    let content = std::fs::read_to_string(&path)?;
                    println!("{}", content);
                } else {
                    let default_str = CliAppConfig::generate_default_config()?;
                    println!("{}", default_str);
                }
            }
        }
        Ok(())
    }

    fn show_path(&self, config_file: Option<String>) -> Result<()> {
        let mut config_path = if let Some(path) = config_file {
            std::path::PathBuf::from(path)
        } else {
            default_config_path()?
        };

        if config_path.extension().is_none() {
            config_path.set_extension("toml");
        }

        println!("{}", config_path.display());
        Ok(())
    }
}
