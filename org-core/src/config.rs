use std::{env, fs, io, path::PathBuf};

use crate::OrgModeError;
use serde::{Deserialize, Serialize};
use shellexpand::tilde;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub org: OrgConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub cli: CliConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgConfig {
    #[serde(default = "default_org_directory")]
    pub org_directory: String,
    #[serde(default = "default_notes_file")]
    pub org_default_notes_file: String,
    #[serde(default = "default_agenda_files")]
    pub org_agenda_files: Vec<String>,
    #[serde(default)]
    pub org_agenda_text_search_extra_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_file")]
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    #[serde(default = "default_output_format")]
    pub default_format: String,
}

fn default_org_directory() -> String {
    "~/org/".to_string()
}

fn default_notes_file() -> String {
    "notes.org".to_string()
}

fn default_agenda_files() -> Vec<String> {
    vec!["agenda.org".to_string()]
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_file() -> String {
    "~/.local/share/org-mcp-server/logs/server.log".to_string()
}

fn default_output_format() -> String {
    "plain".to_string()
}

impl Default for OrgConfig {
    fn default() -> Self {
        Self {
            org_directory: default_org_directory(),
            org_default_notes_file: default_notes_file(),
            org_agenda_files: default_agenda_files(),
            org_agenda_text_search_extra_files: Vec::default(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: default_log_file(),
        }
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            default_format: default_output_format(),
        }
    }
}

#[derive(Debug)]
pub struct ConfigBuilder {
    config: Config,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn with_config_file(mut self, config_path: Option<&str>) -> Result<Self, OrgModeError> {
        let config_file = if let Some(path) = config_path {
            PathBuf::from(path)
        } else {
            self.default_config_path()?
        };

        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file).map_err(|e| {
                OrgModeError::ConfigError(format!(
                    "Failed to read config file {config_file:?}: {e}"
                ))
            })?;

            self.config = toml::from_str(&content).map_err(|e| {
                OrgModeError::ConfigError(format!(
                    "Failed to parse config file {config_file:?}: {e}"
                ))
            })?;
        }

        Ok(self)
    }

    pub fn with_env_vars(mut self) -> Self {
        if let Ok(root_dir) = env::var("ORG_ROOT_DIRECTORY") {
            self.config.org.org_directory = root_dir;
        }

        if let Ok(notes_file) = env::var("ORG_DEFAULT_NOTES_FILE") {
            self.config.org.org_default_notes_file = notes_file;
        }

        if let Ok(agenda_files) = env::var("ORG_AGENDA_FILES") {
            self.config.org.org_agenda_files = agenda_files
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        if let Ok(extra_files) = env::var("ORG_AGENDA_TEXT_SEARCH_EXTRA_FILES") {
            self.config.org.org_agenda_text_search_extra_files = extra_files
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        if let Ok(log_level) = env::var("ORG_LOG_LEVEL") {
            self.config.logging.level = log_level;
        }

        if let Ok(log_file) = env::var("ORG_LOG_FILE") {
            self.config.logging.file = log_file;
        }

        self
    }

    pub fn with_cli_overrides(
        mut self,
        root_directory: Option<String>,
        log_level: Option<String>,
    ) -> Self {
        if let Some(root_dir) = root_directory {
            self.config.org.org_directory = root_dir;
        }

        if let Some(level) = log_level {
            self.config.logging.level = level;
        }

        self
    }

    pub fn build(self) -> Config {
        self.config
    }

    fn default_config_path(&self) -> Result<PathBuf, OrgModeError> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            OrgModeError::ConfigError("Could not determine config directory".to_string())
        })?;

        Ok(config_dir.join("org-mcp-server.toml"))
    }
}

impl Config {
    pub fn load() -> Result<Self, OrgModeError> {
        ConfigBuilder::new()
            .with_config_file(None)?
            .with_env_vars()
            .build()
            .validate()
    }

    pub fn load_with_overrides(
        config_file: Option<String>,
        root_directory: Option<String>,
        log_level: Option<String>,
    ) -> Result<Self, OrgModeError> {
        ConfigBuilder::new()
            .with_config_file(config_file.as_deref())?
            .with_env_vars()
            .with_cli_overrides(root_directory, log_level)
            .build()
            .validate()
    }

    pub fn validate(mut self) -> Result<Self, OrgModeError> {
        let expanded_root = tilde(&self.org.org_directory);
        let root_path = PathBuf::from(expanded_root.as_ref());

        if !root_path.exists() {
            return Err(OrgModeError::ConfigError(format!(
                "Root directory does not exist: {}",
                self.org.org_directory
            )));
        }

        if !root_path.is_dir() {
            return Err(OrgModeError::ConfigError(format!(
                "Root directory is not a directory: {}",
                self.org.org_directory
            )));
        }

        match fs::read_dir(&root_path) {
            Ok(_) => {}
            Err(e) => {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    return Err(OrgModeError::InvalidDirectory(format!(
                        "Permission denied accessing directory: {root_path:?}"
                    )));
                }
                return Err(OrgModeError::IoError(e));
            }
        }

        self.org.org_directory = expanded_root.to_string();
        Ok(self)
    }

    pub fn generate_default_config() -> Result<String, OrgModeError> {
        let config = Config::default();
        toml::to_string_pretty(&config).map_err(|e| {
            OrgModeError::ConfigError(format!("Failed to serialize default config: {e}"))
        })
    }

    pub fn default_config_path() -> Result<PathBuf, OrgModeError> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            OrgModeError::ConfigError("Could not determine config directory".to_string())
        })?;

        Ok(config_dir.join("org-mcp-server.toml"))
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), OrgModeError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                OrgModeError::ConfigError(format!("Failed to create config directory: {e}"))
            })?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| OrgModeError::ConfigError(format!("Failed to serialize config: {e}")))?;

        std::fs::write(path, content)
            .map_err(|e| OrgModeError::ConfigError(format!("Failed to write config file: {e}")))?;

        Ok(())
    }
}

impl OrgConfig {
    pub fn validate(mut self) -> Result<Self, OrgModeError> {
        let expanded_root = tilde(&self.org_directory);
        let root_path = PathBuf::from(expanded_root.as_ref());

        if !root_path.exists() {
            return Err(OrgModeError::ConfigError(format!(
                "Root directory does not exist: {}",
                self.org_directory
            )));
        }

        if !root_path.is_dir() {
            return Err(OrgModeError::ConfigError(format!(
                "Root directory is not a directory: {}",
                self.org_directory
            )));
        }

        match fs::read_dir(&root_path) {
            Ok(_) => {}
            Err(e) => {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    return Err(OrgModeError::InvalidDirectory(format!(
                        "Permission denied accessing directory: {root_path:?}"
                    )));
                }
                return Err(OrgModeError::IoError(e));
            }
        }

        self.org_directory = expanded_root.to_string();
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use temp_env::with_vars;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.org.org_directory, "~/org/");
        assert_eq!(config.org.org_default_notes_file, "notes.org");
        assert_eq!(config.org.org_agenda_files, vec!["agenda.org"]);
        assert!(config.org.org_agenda_text_search_extra_files.is_empty());
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.cli.default_format, "plain");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.org.org_directory, parsed.org.org_directory);
        assert_eq!(
            config.org.org_default_notes_file,
            parsed.org.org_default_notes_file
        );
        assert_eq!(config.org.org_agenda_files, parsed.org.org_agenda_files);
    }

    #[test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "Environment variable handling unreliable in Windows tests"
    )]
    #[serial]
    fn test_env_var_override() {
        with_vars(
            [
                ("ORG_ROOT_DIRECTORY", Some("/tmp/test-org")),
                ("ORG_DEFAULT_NOTES_FILE", Some("test-notes.org")),
                ("ORG_AGENDA_FILES", Some("agenda1.org,agenda2.org")),
            ],
            || {
                let config = ConfigBuilder::new().with_env_vars().build();

                assert_eq!(config.org.org_directory, "/tmp/test-org");
                assert_eq!(config.org.org_default_notes_file, "test-notes.org");
                assert_eq!(
                    config.org.org_agenda_files,
                    vec!["agenda1.org", "agenda2.org"]
                );
            },
        );
    }

    #[test]
    fn test_cli_override() {
        let config = ConfigBuilder::new()
            .with_cli_overrides(Some("/custom/org".to_string()), Some("debug".to_string()))
            .build();

        assert_eq!(config.org.org_directory, "/custom/org");
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_config_file_loading() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test-config.toml");

        let test_config = r#"
[org]
org_directory = "/test/org"
org_default_notes_file = "custom-notes.org"
org_agenda_files = ["test1.org", "test2.org"]

[logging]
level = "debug"

[cli]
default_format = "json"
"#;

        std::fs::write(&config_path, test_config).unwrap();

        let config = ConfigBuilder::new()
            .with_config_file(Some(config_path.to_str().unwrap()))
            .unwrap()
            .build();

        assert_eq!(config.org.org_directory, "/test/org");
        assert_eq!(config.org.org_default_notes_file, "custom-notes.org");
        assert_eq!(config.org.org_agenda_files, vec!["test1.org", "test2.org"]);
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.cli.default_format, "json");
    }

    #[test]
    fn test_validate_directory_expansion() {
        let temp_dir = tempdir().unwrap();
        let mut config = Config::default();
        config.org.org_directory = temp_dir.path().to_str().unwrap().to_string();

        let validated = config.validate().unwrap();
        assert_eq!(
            validated.org.org_directory,
            temp_dir.path().to_str().unwrap()
        );
    }

    #[test]
    fn test_validate_nonexistent_directory() {
        let mut config = Config::default();
        config.org.org_directory = "/nonexistent/test/directory".to_string();

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("Root directory does not exist"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_validate_non_directory_path() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("not-a-dir.txt");
        std::fs::write(&file_path, "test").unwrap();

        let mut config = Config::default();
        config.org.org_directory = file_path.to_str().unwrap().to_string();

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("not a directory"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    #[serial]
    fn test_load_full_path() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Convert path to forward slashes for TOML compatibility on Windows
        let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
        let test_config = format!(
            r#"
[org]
org_directory = "{}"
"#,
            path_str
        );

        std::fs::write(&config_path, test_config).unwrap();

        with_vars(
            [
                ("XDG_CONFIG_HOME", temp_dir.path().to_str()),
                ("HOME", temp_dir.path().to_str()),
            ],
            || {
                let config = ConfigBuilder::new()
                    .with_config_file(Some(config_path.to_str().unwrap()))
                    .unwrap()
                    .with_env_vars()
                    .build()
                    .validate()
                    .unwrap();

                assert_eq!(config.org.org_directory, path_str);
            },
        );
    }

    #[test]
    #[serial]
    fn test_load_with_overrides_full_hierarchy() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Convert path to forward slashes for TOML compatibility on Windows
        let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
        let test_config = format!(
            r#"
[org]
org_directory = "{}"

[logging]
level = "debug"
"#,
            path_str
        );

        std::fs::write(&config_path, test_config).unwrap();

        with_vars([("ORG_ROOT_DIRECTORY", None::<&str>)], || {
            let config = Config::load_with_overrides(
                Some(config_path.to_str().unwrap().to_string()),
                None,
                Some("trace".to_string()),
            )
            .unwrap();

            assert_eq!(config.org.org_directory, path_str);
            assert_eq!(config.logging.level, "trace");
        });
    }

    #[test]
    fn test_generate_default_config() {
        let toml_str = Config::generate_default_config().unwrap();
        assert!(toml_str.contains("org_directory"));
        assert!(toml_str.contains("~/org/"));
        assert!(toml_str.contains("[logging]"));
        assert!(toml_str.contains("[cli]"));

        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.org.org_directory, "~/org/");
    }

    #[test]
    fn test_save_to_file() {
        let temp_dir = tempdir().unwrap();
        let nested_path = temp_dir.path().join("nested").join("config.toml");

        let mut config = Config::default();
        config.org.org_directory = temp_dir.path().to_str().unwrap().to_string();

        config.save_to_file(&nested_path).unwrap();

        assert!(nested_path.exists());
        let content = std::fs::read_to_string(&nested_path).unwrap();
        assert!(content.contains("org_directory"));
    }

    #[test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "Environment variable handling unreliable in Windows tests"
    )]
    #[serial]
    fn test_env_var_all_fields() {
        with_vars(
            [
                ("ORG_ROOT_DIRECTORY", Some("/tmp/test-org")),
                ("ORG_DEFAULT_NOTES_FILE", Some("test-notes.org")),
                ("ORG_AGENDA_FILES", Some("agenda1.org,agenda2.org")),
                (
                    "ORG_AGENDA_TEXT_SEARCH_EXTRA_FILES",
                    Some("archive.org,old.org"),
                ),
                ("ORG_LOG_LEVEL", Some("trace")),
                ("ORG_LOG_FILE", Some("/tmp/test.log")),
            ],
            || {
                let config = ConfigBuilder::new().with_env_vars().build();

                assert_eq!(config.org.org_directory, "/tmp/test-org");
                assert_eq!(config.org.org_default_notes_file, "test-notes.org");
                assert_eq!(
                    config.org.org_agenda_files,
                    vec!["agenda1.org", "agenda2.org"]
                );
                assert_eq!(
                    config.org.org_agenda_text_search_extra_files,
                    vec!["archive.org", "old.org"]
                );
                assert_eq!(config.logging.level, "trace");
                assert_eq!(config.logging.file, "/tmp/test.log");
            },
        );
    }

    #[test]
    fn test_invalid_toml_syntax() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");

        std::fs::write(&config_path, "invalid toml [ syntax").unwrap();

        let result = ConfigBuilder::new().with_config_file(Some(config_path.to_str().unwrap()));

        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("Failed to parse config file"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_config_file_read_error() {
        let result = ConfigBuilder::new().with_config_file(Some("/nonexistent/path/config.toml"));

        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_config_directory_fallback() {
        let result = ConfigBuilder::new().with_config_file(None);

        assert!(result.is_ok());
    }
}
