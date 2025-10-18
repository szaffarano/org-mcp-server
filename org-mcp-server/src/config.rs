use config::{Config as ConfigRs, ConfigError};
use org_core::{
    LoggingConfig, OrgConfig, OrgModeError,
    config::{build_config_with_file_and_env, load_logging_config, load_org_config},
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
        let builder = ConfigRs::builder().set_default(
            "server.max_connections",
            default_max_connections().to_string(),
        )?;

        let config = build_config_with_file_and_env(config_file, builder)?;

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
    #[serial_test::serial]
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
    #[serial_test::serial]
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

    #[test]
    fn test_save_to_file() {
        let temp_dir = tempdir().unwrap();
        let save_path = temp_dir.path().join("saved_config.toml");

        let config_dir = tempdir().unwrap();
        let config = ServerAppConfig {
            org: OrgConfig {
                org_directory: config_dir.path().to_str().unwrap().to_string(),
                org_default_notes_file: "test.org".to_string(),
                org_agenda_files: vec!["agenda.org".to_string()],
                org_agenda_text_search_extra_files: vec![],
            },
            server: ServerConfig {
                max_connections: 25,
            },
            logging: LoggingConfig {
                level: "warn".to_string(),
                file: "/tmp/server.log".to_string(),
            },
        };

        let result = config.save_to_file(&save_path);
        assert!(result.is_ok());

        assert!(save_path.exists());

        let content = std::fs::read_to_string(&save_path).unwrap();
        assert!(content.contains("max_connections = 25"));
        assert!(content.contains("level = \"warn\""));
        assert!(content.contains("org_default_notes_file = \"test.org\""));
    }

    #[test]
    fn test_save_to_file_creates_parent_directory() {
        let temp_dir = tempdir().unwrap();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("dirs")
            .join("config.toml");

        let config_dir = tempdir().unwrap();
        let config = ServerAppConfig {
            org: OrgConfig {
                org_directory: config_dir.path().to_str().unwrap().to_string(),
                ..OrgConfig::default()
            },
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
        };

        let result = config.save_to_file(&nested_path);
        assert!(result.is_ok());
        assert!(nested_path.exists());
        assert!(nested_path.parent().unwrap().exists());
    }

    #[test]
    #[serial_test::serial]
    #[cfg_attr(
        target_os = "windows",
        ignore = "Environment variable handling unreliable in Windows tests"
    )]
    fn test_env_var_server_override() {
        use temp_env::with_vars;

        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        with_vars(
            [
                ("ORG_ORG__ORG_DIRECTORY", Some(temp_dir_path)),
                ("ORG_SERVER__MAX_CONNECTIONS", Some("50")),
            ],
            || {
                let config = ServerAppConfig::load(None, None, None).unwrap();
                assert_eq!(config.org.org_directory, temp_dir_path);
                assert_eq!(config.server.max_connections, 50);
            },
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_load_server_config_extension_fallback() {
        let temp_dir = tempdir().unwrap();
        let config_dir = temp_dir.path().join(".config");
        std::fs::create_dir_all(&config_dir).unwrap();

        let yaml_config = r#"
server:
  max_connections: 15
org:
  org_directory: "/tmp"
logging:
  level: "info"
"#;

        let yaml_path = config_dir.join("config.yaml");
        std::fs::write(&yaml_path, yaml_config).unwrap();

        let org_dir = tempdir().unwrap();
        let config = ServerAppConfig::load(
            Some(config_dir.join("config").to_str().unwrap().to_string()),
            Some(org_dir.path().to_str().unwrap().to_string()),
            None,
        );

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.server.max_connections, 15);
    }
}
