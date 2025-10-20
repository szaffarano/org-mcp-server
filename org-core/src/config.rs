use std::{fs, io, path::PathBuf};

use crate::OrgModeError;
use config::{
    Config as ConfigRs, ConfigError, Environment, File,
    builder::{ConfigBuilder, DefaultState},
};
use serde::{Deserialize, Serialize};
use shellexpand::tilde;

/// Core org-mode configuration settings
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
    #[serde(default = "default_todo_keywords")]
    pub org_todo_keywords: Vec<String>,
}

/// Logging configuration (shared across CLI and server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_file")]
    pub file: String,
}

impl Default for OrgConfig {
    fn default() -> Self {
        Self {
            org_directory: default_org_directory(),
            org_default_notes_file: default_notes_file(),
            org_agenda_files: default_agenda_files(),
            org_agenda_text_search_extra_files: Vec::default(),
            org_todo_keywords: default_todo_keywords(),
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

impl OrgConfig {
    /// Validate and expand paths in the org configuration
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

        if self.org_todo_keywords.len() < 2 {
            return Err(OrgModeError::ConfigError(
                "org_todo_keywords must contain at least two keywords".to_string(),
            ));
        }

        let separators: Vec<usize> = self
            .org_todo_keywords
            .iter()
            .enumerate()
            .filter_map(|(i, x)| (x == "|").then_some(i))
            .collect();

        if separators.len() > 1 {
            return Err(OrgModeError::ConfigError(
                "Multiple '|' separators found in org_todo_keywords".to_string(),
            ));
        }

        if separators.len() == 1 {
            let sep_pos = separators[0];
            if sep_pos == 0 {
                return Err(OrgModeError::ConfigError(
                    "Separator '|' cannot be at the beginning of org_todo_keywords".to_string(),
                ));
            }
            if sep_pos == self.org_todo_keywords.len() - 1 {
                return Err(OrgModeError::ConfigError(
                    "Separator '|' cannot be at the end of org_todo_keywords".to_string(),
                ));
            }
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

    pub fn unfinished_keywords(&self) -> Vec<String> {
        if let Some(pos) = self.org_todo_keywords.iter().position(|x| x == "|") {
            self.org_todo_keywords[..pos].to_vec()
        } else {
            self.org_todo_keywords[..self.org_todo_keywords.len() - 1].to_vec()
        }
    }

    pub fn finished_keywords(&self) -> Vec<String> {
        if let Some(pos) = self.org_todo_keywords.iter().position(|x| x == "|") {
            self.org_todo_keywords[pos + 1..self.org_todo_keywords.len()].to_vec()
        } else {
            self.org_todo_keywords
                .last()
                .map(|e| vec![e.clone()])
                .unwrap_or_default()
        }
    }
}

/// Get the default configuration file path
pub fn default_config_path() -> Result<PathBuf, OrgModeError> {
    Ok(default_config_dir()?.join("config"))
}

/// Find an existing config file, trying common extensions if the base path doesn't exist
///
/// Returns the first existing config file path found, or None if no config file exists.
/// Tries extensions in order: toml, yaml, yml, json
pub fn find_config_file(base_path: PathBuf) -> Option<PathBuf> {
    if base_path.exists() {
        return Some(base_path);
    }

    if let Some(parent) = base_path.parent() {
        for ext in &["toml", "yaml", "yml", "json"] {
            let path_with_ext = parent.join(format!("config.{ext}"));
            if path_with_ext.exists() {
                return Some(path_with_ext);
            }
        }
    }

    None
}

/// Build config from file and environment sources
///
/// Takes a builder with defaults already set, adds file and env sources,
/// then builds and returns the config ready for section extraction.
///
/// # Arguments
/// * `config_file` - Optional path to config file
/// * `builder` - ConfigBuilder with defaults already set
///
/// # Returns
/// Built Config object ready for section extraction via .get()
pub fn build_config_with_file_and_env(
    config_file: Option<&str>,
    builder: ConfigBuilder<DefaultState>,
) -> Result<ConfigRs, OrgModeError> {
    let config_path = if let Some(path) = config_file {
        PathBuf::from(path)
    } else {
        default_config_path()?
    };

    let mut builder = builder;
    if let Some(config_file_path) = find_config_file(config_path) {
        builder = builder.add_source(File::from(config_file_path).required(false));
    }

    builder = builder.add_source(
        Environment::with_prefix("ORG")
            .prefix_separator("_")
            .separator("__"),
    );

    builder
        .build()
        .map_err(|e: ConfigError| OrgModeError::ConfigError(format!("Failed to build config: {e}")))
}

/// Load org configuration using config-rs with layered sources
pub fn load_org_config(
    config_file: Option<&str>,
    org_directory: Option<&str>,
) -> Result<OrgConfig, OrgModeError> {
    let builder = ConfigRs::builder()
        .set_default("org.org_directory", default_org_directory())?
        .set_default("org.org_default_notes_file", default_notes_file())?
        .set_default("org.org_agenda_files", default_agenda_files())?;

    let config = build_config_with_file_and_env(config_file, builder)?;

    let mut org_config: OrgConfig = config.get("org").map_err(|e: ConfigError| {
        OrgModeError::ConfigError(format!("Failed to deserialize org config: {e}"))
    })?;

    if let Some(org_directory) = org_directory {
        org_config.org_directory = org_directory.to_string();
    }

    org_config.validate()
}

/// Load logging configuration using config-rs
pub fn load_logging_config(
    config_file: Option<&str>,
    log_level: Option<&str>,
) -> Result<LoggingConfig, OrgModeError> {
    let builder = ConfigRs::builder()
        .set_default("logging.level", default_log_level())?
        .set_default("logging.file", default_log_file())?;

    let config = build_config_with_file_and_env(config_file, builder)?;

    let mut config: LoggingConfig = config.get("logging").map_err(|e: ConfigError| {
        OrgModeError::ConfigError(format!("Failed to deserialize logging config: {e}"))
    })?;

    if let Some(level) = log_level {
        config.level = level.to_string();
    }

    Ok(config)
}

fn default_config_dir() -> Result<PathBuf, OrgModeError> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        OrgModeError::ConfigError("Could not determine config directory".to_string())
    })?;

    Ok(config_dir.join("org-mcp"))
}

pub fn default_org_directory() -> String {
    "~/org/".to_string()
}

pub fn default_notes_file() -> String {
    "notes.org".to_string()
}

pub fn default_agenda_files() -> Vec<String> {
    vec!["agenda.org".to_string()]
}

pub fn default_todo_keywords() -> Vec<String> {
    vec!["TODO".to_string(), "|".to_string(), "DONE".to_string()]
}

pub fn default_log_level() -> String {
    "info".to_string()
}

pub fn default_log_file() -> String {
    "~/.local/share/org-mcp-server/logs/server.log".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use temp_env::with_vars;
    use tempfile::tempdir;

    #[test]
    fn test_default_org_config() {
        let config = OrgConfig::default();
        assert_eq!(config.org_directory, "~/org/");
        assert_eq!(config.org_default_notes_file, "notes.org");
        assert_eq!(config.org_agenda_files, vec!["agenda.org"]);
        assert!(config.org_agenda_text_search_extra_files.is_empty());
    }

    #[test]
    fn test_default_logging_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.file, "~/.local/share/org-mcp-server/logs/server.log");
    }

    #[test]
    fn test_config_serialization() {
        let org_config = OrgConfig::default();
        let toml_str = toml::to_string_pretty(&org_config).unwrap();
        let parsed: OrgConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(org_config.org_directory, parsed.org_directory);
        assert_eq!(
            org_config.org_default_notes_file,
            parsed.org_default_notes_file
        );
        assert_eq!(org_config.org_agenda_files, parsed.org_agenda_files);
    }

    #[test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "Environment variable handling unreliable in Windows tests"
    )]
    #[serial]
    fn test_env_var_override() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        with_vars(
            [
                ("ORG_ORG__ORG_DIRECTORY", Some(temp_path)),
                ("ORG_ORG__ORG_DEFAULT_NOTES_FILE", Some("test-notes.org")),
            ],
            || {
                let config = load_org_config(None, None).unwrap();
                assert_eq!(config.org_directory, temp_path);
                assert_eq!(config.org_default_notes_file, "test-notes.org");
            },
        );
    }

    #[test]
    fn test_validate_directory_expansion() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            ..OrgConfig::default()
        };

        let validated = config.validate().unwrap();
        assert_eq!(validated.org_directory, temp_dir.path().to_str().unwrap());
    }

    #[test]
    fn test_validate_nonexistent_directory() {
        let config = OrgConfig {
            org_directory: "/nonexistent/test/directory".to_string(),
            ..OrgConfig::default()
        };

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

        let config = OrgConfig {
            org_directory: file_path.to_str().unwrap().to_string(),
            ..OrgConfig::default()
        };

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
    fn test_load_from_toml_file() {
        let temp_dir = tempdir().unwrap();
        let path_str = test_utils::config::normalize_path(temp_dir.path());
        let test_config = format!(
            r#"
[org]
org_directory = "{path_str}"
org_default_notes_file = "custom-notes.org"
org_agenda_files = ["test1.org", "test2.org"]
"#,
        );

        let config_path = test_utils::config::create_toml_config(&temp_dir, &test_config).unwrap();

        let config = load_org_config(Some(config_path.to_str().unwrap()), None);
        let config = config.unwrap();

        assert_eq!(config.org_directory, path_str);
        assert_eq!(config.org_default_notes_file, "custom-notes.org");
        assert_eq!(config.org_agenda_files, vec!["test1.org", "test2.org"]);
    }

    #[test]
    #[serial]
    fn test_load_from_yaml_file() {
        let temp_dir = tempdir().unwrap();
        let path_str = test_utils::config::normalize_path(temp_dir.path());
        let yaml_config = format!(
            r#"
org:
  org_directory: "{path_str}"
  org_default_notes_file: "yaml-notes.org"
  org_agenda_files:
    - "yaml1.org"
    - "yaml2.org"
"#
        );

        let yaml_path = test_utils::config::create_yaml_config(&temp_dir, &yaml_config).unwrap();
        let config_dir = yaml_path.parent().unwrap();

        let config = load_org_config(Some(config_dir.join("config").to_str().unwrap()), None);
        let config = config.unwrap();

        assert_eq!(config.org_directory, path_str);
        assert_eq!(config.org_default_notes_file, "yaml-notes.org");
        assert_eq!(config.org_agenda_files, vec!["yaml1.org", "yaml2.org"]);
    }

    #[test]
    #[serial]
    fn test_load_from_yml_file() {
        let temp_dir = tempdir().unwrap();
        let path_str = test_utils::config::normalize_path(temp_dir.path());
        let yml_config = format!(
            r#"
org:
  org_directory: "{path_str}"
  org_default_notes_file: "yml-notes.org"
logging:
  level: "debug"
  file: "/tmp/test.log"
"#
        );

        let yml_path = test_utils::config::create_yml_config(&temp_dir, &yml_config).unwrap();
        let config_dir = yml_path.parent().unwrap();

        let config = load_org_config(Some(config_dir.join("config").to_str().unwrap()), None);
        let config = config.unwrap();
        assert_eq!(config.org_default_notes_file, "yml-notes.org");

        let logging_config =
            load_logging_config(Some(config_dir.join("config").to_str().unwrap()), None);
        let logging_config = logging_config.unwrap();
        assert_eq!(logging_config.level, "debug");
        assert_eq!(logging_config.file, "/tmp/test.log");
    }

    #[test]
    #[serial]
    fn test_load_from_json_file() {
        let temp_dir = tempdir().unwrap();
        let path_str = test_utils::config::normalize_path(temp_dir.path());
        let json_config = format!(
            r#"{{
  "org": {{
    "org_directory": "{path_str}",
    "org_default_notes_file": "json-notes.org",
    "org_agenda_files": ["json1.org", "json2.org"]
  }}
}}"#
        );

        let json_path = test_utils::config::create_json_config(&temp_dir, &json_config).unwrap();
        let config_dir = json_path.parent().unwrap();

        let config = load_org_config(Some(config_dir.join("config").to_str().unwrap()), None);
        let config = config.unwrap();

        assert_eq!(config.org_directory, path_str);
        assert_eq!(config.org_default_notes_file, "json-notes.org");
        assert_eq!(config.org_agenda_files, vec!["json1.org", "json2.org"]);
    }

    #[test]
    #[serial]
    fn test_logging_config_file_extensions() {
        let temp_dir = tempdir().unwrap();

        let toml_config = r#"
[logging]
level = "trace"
file = "/var/log/test.log"
"#;

        let toml_path = test_utils::config::create_toml_config(&temp_dir, toml_config).unwrap();

        let config = load_logging_config(Some(toml_path.to_str().unwrap()), None);
        let config = config.unwrap();

        assert_eq!(config.level, "trace");
        assert_eq!(config.file, "/var/log/test.log");
    }

    #[test]
    fn test_unfinished_keywords_with_separator() {
        let config = OrgConfig {
            org_todo_keywords: vec![
                "TODO".to_string(),
                "IN_PROGRESS".to_string(),
                "|".to_string(),
                "DONE".to_string(),
                "CANCELLED".to_string(),
            ],
            ..OrgConfig::default()
        };

        let unfinished = config.unfinished_keywords();
        assert_eq!(unfinished, vec!["TODO", "IN_PROGRESS"]);
    }

    #[test]
    fn test_unfinished_keywords_without_separator() {
        let config = OrgConfig {
            org_todo_keywords: vec![
                "TODO".to_string(),
                "IN_PROGRESS".to_string(),
                "DONE".to_string(),
            ],
            ..OrgConfig::default()
        };

        let unfinished = config.unfinished_keywords();
        assert_eq!(unfinished, vec!["TODO", "IN_PROGRESS"]);
    }

    #[test]
    fn test_finished_keywords_with_separator() {
        let config = OrgConfig {
            org_todo_keywords: vec![
                "TODO".to_string(),
                "|".to_string(),
                "DONE".to_string(),
                "CANCELLED".to_string(),
            ],
            ..OrgConfig::default()
        };

        let finished = config.finished_keywords();
        assert_eq!(finished, vec!["DONE", "CANCELLED"]);
    }

    #[test]
    fn test_finished_keywords_without_separator() {
        let config = OrgConfig {
            org_todo_keywords: vec!["TODO".to_string(), "DONE".to_string()],
            ..OrgConfig::default()
        };

        let finished = config.finished_keywords();
        assert_eq!(finished, vec!["DONE"]);
    }

    #[test]
    fn test_validate_empty_keywords() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec![],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("must contain at least two keywords"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_validate_single_keyword() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec!["TODO".to_string()],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("must contain at least two keywords"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_validate_multiple_separators() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec![
                "TODO".to_string(),
                "|".to_string(),
                "DONE".to_string(),
                "|".to_string(),
                "CANCELLED".to_string(),
            ],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("Multiple '|' separators"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_validate_separator_at_beginning() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec!["|".to_string(), "DONE".to_string()],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("cannot be at the beginning"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_validate_separator_at_end() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec!["TODO".to_string(), "|".to_string()],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("cannot be at the end"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_validate_only_separator() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec!["|".to_string()],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OrgModeError::ConfigError(msg) => {
                assert!(msg.contains("must contain at least two keywords"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_validate_valid_keywords_with_separator() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec!["TODO".to_string(), "|".to_string(), "DONE".to_string()],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_valid_keywords_without_separator() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec!["TODO".to_string(), "DONE".to_string()],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_multiple_unfinished_and_finished() {
        let temp_dir = tempdir().unwrap();
        let config = OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_todo_keywords: vec![
                "TODO".to_string(),
                "IN_PROGRESS".to_string(),
                "|".to_string(),
                "DONE".to_string(),
                "CANCELLED".to_string(),
            ],
            ..OrgConfig::default()
        };

        let result = config.validate();
        assert!(result.is_ok());
    }
}
