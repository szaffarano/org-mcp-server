use crate::config::CliAppConfig;
use anyhow::Result;
use clap::{Args, Subcommand};
use org_core::config::{default_config_path, find_config_file};

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

                if let Some(path) = find_config_file(config_path) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_show_config_when_file_doesnt_exist() {
        let temp_dir = tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent_config");

        let cmd = ConfigCommand {
            action: ConfigAction::Show,
        };

        let result = cmd.show_config(Some(nonexistent_path.to_str().unwrap().to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_config_with_existing_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let test_config = format!(
            r#"
[org]
org_directory = "{}"

[cli]
default_format = "json"
"#,
            temp_dir.path().to_str().unwrap().replace('\\', "/")
        );

        std::fs::write(&config_path, test_config).unwrap();

        let cmd = ConfigCommand {
            action: ConfigAction::Show,
        };

        let result = cmd.show_config(Some(config_path.to_str().unwrap().to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_path_adds_extension() {
        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path().join("config");

        let cmd = ConfigCommand {
            action: ConfigAction::Path,
        };

        let result = cmd.show_path(Some(base_path.to_str().unwrap().to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_config_creates_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("new_config.toml");

        let cmd = ConfigCommand {
            action: ConfigAction::Init,
        };

        let result = cmd.init_config(Some(config_path.to_str().unwrap().to_string()));
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    #[test]
    fn test_init_config_file_already_exists() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("existing_config.toml");

        std::fs::write(&config_path, "test content").unwrap();

        let cmd = ConfigCommand {
            action: ConfigAction::Init,
        };

        let result = cmd.init_config(Some(config_path.to_str().unwrap().to_string()));
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_show_config_extension_fallback() {
        let temp_dir = tempdir().unwrap();
        let config_dir = temp_dir.path().join(".config");
        std::fs::create_dir_all(&config_dir).unwrap();

        let yaml_config = format!(
            r#"
org:
  org_directory: "{}"
cli:
  default_format: "json"
"#,
            temp_dir.path().to_str().unwrap().replace('\\', "/")
        );

        let yaml_path = config_dir.join("config.yaml");
        std::fs::write(&yaml_path, yaml_config).unwrap();

        let cmd = ConfigCommand {
            action: ConfigAction::Show,
        };

        let result = cmd.show_config(Some(
            config_dir.join("config").to_str().unwrap().to_string(),
        ));
        assert!(result.is_ok());
    }
}
