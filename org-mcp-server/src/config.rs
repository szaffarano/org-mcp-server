use config::{Config as ConfigRs, ConfigError, Environment, File};
use org_core::{
    LoggingConfig, OrgConfig, OrgModeError,
    config::{default_config_path, load_logging_config, load_org_config},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// MCP server-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
        }
    }
}

/// Complete MCP server application configuration
#[derive(Debug, Clone)]
pub struct ServerAppConfig {
    pub org: OrgConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
}

impl ServerAppConfig {
    /// Load server configuration from file and environment with CLI argument overrides
    pub fn load(
        config_file: Option<String>,
        root_directory: Option<String>,
        log_level: Option<String>,
    ) -> Result<Self, OrgModeError> {
        let org = load_org_config(config_file.as_deref(), root_directory.as_deref())?;
        let server = Self::load_server_config(config_file.as_deref())?;
        let logging = load_logging_config(config_file.as_deref(), log_level.as_deref())?;

        Ok(Self {
            org,
            server,
            logging,
        })
    }

    fn load_server_config(config_file: Option<&str>) -> Result<ServerConfig, OrgModeError> {
        let mut builder = ConfigRs::builder().set_default(
            "server.max_connections",
            default_max_connections().to_string(),
        )?;

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

        let server_config: ServerConfig = config.get("server").map_err(|e: ConfigError| {
            OrgModeError::ConfigError(format!("Failed to deserialize server config: {e}"))
        })?;

        Ok(server_config)
    }

    /// Save the configuration to a file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), OrgModeError> {
        #[derive(Serialize)]
        struct SavedConfig<'a> {
            org: &'a OrgConfig,
            server: &'a ServerConfig,
            logging: &'a LoggingConfig,
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                OrgModeError::ConfigError(format!("Failed to create config directory: {e}"))
            })?;
        }

        let saved = SavedConfig {
            org: &self.org,
            server: &self.server,
            logging: &self.logging,
        };

        let content = toml::to_string_pretty(&saved)
            .map_err(|e| OrgModeError::ConfigError(format!("Failed to serialize config: {e}")))?;

        std::fs::write(path, content)
            .map_err(|e| OrgModeError::ConfigError(format!("Failed to write config file: {e}")))?;

        Ok(())
    }
}

fn default_max_connections() -> usize {
    10
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_server_config() {
        let config = ServerConfig::default();
        assert_eq!(config.max_connections, 10);
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

[server]
max_connections = 20

[logging]
level = "debug"
"#,
            path_str
        );

        std::fs::write(&config_path, test_config).unwrap();

        let config =
            ServerAppConfig::load(Some(config_path.to_str().unwrap().to_string()), None, None)
                .unwrap();

        assert_eq!(config.org.org_directory, path_str);
        assert_eq!(config.server.max_connections, 20);
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

[server]
max_connections = 5
"#,
            temp_dir.path().to_str().unwrap().replace('\\', "/")
        );

        std::fs::write(&config_path, test_config).unwrap();

        let override_dir = tempdir().unwrap();
        let config = ServerAppConfig::load(
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
