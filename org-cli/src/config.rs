use config::{Config as ConfigRs, ConfigError, Environment, File};
use org_core::{
    LoggingConfig, OrgConfig, OrgModeError,
    config::{default_config_path, load_logging_config, load_org_config},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CLI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    #[serde(default = "default_output_format")]
    pub default_format: String,
}

fn default_output_format() -> String {
    "plain".to_string()
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            default_format: default_output_format(),
        }
    }
}

/// Complete CLI application configuration
#[derive(Debug, Clone)]
pub struct CliAppConfig {
    pub org: OrgConfig,
    pub cli: CliConfig,
    pub logging: LoggingConfig,
}

impl CliAppConfig {
    /// Load CLI configuration from file and environment with CLI argument overrides
    pub fn load(
        config_file: Option<String>,
        root_directory: Option<String>,
        log_level: Option<String>,
    ) -> Result<Self, OrgModeError> {
        let org = load_org_config(config_file.as_deref(), root_directory.as_deref())?;
        let cli = Self::load_cli_config(config_file.as_deref())?;
        let logging = load_logging_config(config_file.as_deref(), log_level.as_deref())?;

        Ok(Self { org, cli, logging })
    }

    pub fn load_cli_config(config_file: Option<&str>) -> Result<CliConfig, OrgModeError> {
        let mut builder = ConfigRs::builder().set_default("cli.default_format", "plain")?;

        let config_path = config_file
            .map(PathBuf::from)
            .unwrap_or_else(|| default_config_path().expect("Failed to get default config path"));

        if config_path.exists() {
            builder = builder.add_source(File::from(config_path).required(false));
        } else if let Some(parent) = config_path.parent() {
            for ext in &["toml", "yaml", "yml", "json"] {
                let path_with_ext = parent.join(format!("config.{ext}"));
                if path_with_ext.exists() {
                    builder = builder.add_source(File::from(path_with_ext).required(false));
                    break;
                }
            }
        }

        builder = builder.add_source(
            Environment::with_prefix("ORG")
                .prefix_separator("_")
                .separator("__"),
        );

        let config = builder.build().map_err(|e: ConfigError| {
            OrgModeError::ConfigError(format!("Failed to build config: {e}"))
        })?;

        config.get("cli").map_err(|e: ConfigError| {
            OrgModeError::ConfigError(format!("Failed to deserialize org config: {e}"))
        })
    }

    /// Generate a default configuration as TOML string
    pub fn generate_default_config() -> Result<String, OrgModeError> {
        #[derive(Serialize)]
        struct DefaultConfig {
            org: OrgConfig,
            cli: CliConfig,
            logging: LoggingConfig,
        }

        let config = DefaultConfig {
            org: OrgConfig::default(),
            cli: CliConfig::default(),
            logging: LoggingConfig::default(),
        };

        toml::to_string_pretty(&config).map_err(|e| {
            OrgModeError::ConfigError(format!("Failed to serialize default config: {e}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_cli_config() {
        let config = CliConfig::default();
        assert_eq!(config.default_format, "plain");
    }

    #[test]
    fn test_generate_default_config() {
        let toml_str = CliAppConfig::generate_default_config().unwrap();
        assert!(toml_str.contains("org_directory"));
        assert!(toml_str.contains("[cli]"));
        assert!(toml_str.contains("[logging]"));
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
        let test_config = format!(
            r#"
[org]
org_directory = "{}"

[cli]
default_format = "json"

[logging]
level = "debug"
"#,
            path_str
        );

        std::fs::write(&config_path, test_config).unwrap();

        let config =
            CliAppConfig::load(Some(config_path.to_str().unwrap().to_string()), None, None)
                .unwrap();

        assert_eq!(config.org.org_directory, path_str);
        assert_eq!(config.cli.default_format, "json");
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_cli_overrides() {
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

        let override_dir = tempdir().unwrap();
        let config = CliAppConfig::load(
            Some(config_path.to_str().unwrap().to_string()),
            Some(override_dir.path().to_str().unwrap().to_string()),
            Some("trace".to_string()),
        )
        .unwrap();

        assert_eq!(
            config.org.org_directory,
            override_dir.path().to_str().unwrap()
        );
        assert_eq!(config.logging.level, "trace");
    }
}
