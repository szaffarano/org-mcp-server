//! Test utilities for configuration file testing.
//!
//! This module provides helper functions to create temporary configuration files
//! for testing config loading across different crates.

use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Normalize a path for use in config files (handles Windows backslashes).
///
/// Converts backslashes to forward slashes for cross-platform compatibility
/// in configuration files.
///
/// # Arguments
/// * `path` - The path to normalize
///
/// # Returns
/// * Normalized path string
pub fn normalize_path(path: &Path) -> String {
    path.to_str().unwrap().replace('\\', "/")
}

/// Create a temporary TOML config file with the given content.
///
/// # Arguments
/// * `temp_dir` - Temporary directory to create the config in
/// * `content` - TOML content as a string
///
/// # Returns
/// * Path to the created config file
///
/// # Example
/// ```no_run
/// use tempfile::TempDir;
/// use test_utils::config::create_toml_config;
///
/// let temp_dir = TempDir::new()?;
/// let config_path = create_toml_config(&temp_dir, r#"
/// [org]
/// org_directory = "/tmp"
/// "#)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn create_toml_config(
    temp_dir: &TempDir,
    content: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, content)?;
    Ok(config_path)
}

/// Create a temporary YAML config file with the given content.
///
/// # Arguments
/// * `temp_dir` - Temporary directory to create the config in
/// * `content` - YAML content as a string
///
/// # Returns
/// * Path to the created config file
pub fn create_yaml_config(
    temp_dir: &TempDir,
    content: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = temp_dir.path().join(".config");
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yaml");
    std::fs::write(&config_path, content)?;
    Ok(config_path)
}

/// Create a temporary YML config file with the given content.
///
/// # Arguments
/// * `temp_dir` - Temporary directory to create the config in
/// * `content` - YML content as a string
///
/// # Returns
/// * Path to the created config file
pub fn create_yml_config(
    temp_dir: &TempDir,
    content: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = temp_dir.path().join(".config");
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yml");
    std::fs::write(&config_path, content)?;
    Ok(config_path)
}

/// Create a temporary JSON config file with the given content.
///
/// # Arguments
/// * `temp_dir` - Temporary directory to create the config in
/// * `content` - JSON content as a string
///
/// # Returns
/// * Path to the created config file
pub fn create_json_config(
    temp_dir: &TempDir,
    content: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = temp_dir.path().join(".config");
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.json");
    std::fs::write(&config_path, content)?;
    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        let path = Path::new("/tmp/test");
        assert_eq!(normalize_path(path), "/tmp/test");

        #[cfg(windows)]
        {
            let path = Path::new("C:\\Users\\test");
            assert_eq!(normalize_path(path), "C:/Users/test");
        }
    }

    #[test]
    fn test_create_toml_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_toml_config(&temp_dir, "[test]\nvalue = 42").unwrap();

        assert!(config_path.exists());
        assert_eq!(config_path.file_name().unwrap(), "config.toml");

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[test]"));
        assert!(content.contains("value = 42"));
    }

    #[test]
    fn test_create_yaml_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_yaml_config(&temp_dir, "test:\n  value: 42").unwrap();

        assert!(config_path.exists());
        assert_eq!(config_path.file_name().unwrap(), "config.yaml");

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("test:"));
        assert!(content.contains("value: 42"));
    }

    #[test]
    fn test_create_yml_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_yml_config(&temp_dir, "test:\n  value: 42").unwrap();

        assert!(config_path.exists());
        assert_eq!(config_path.file_name().unwrap(), "config.yml");
    }

    #[test]
    fn test_create_json_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_json_config(&temp_dir, r#"{"test": {"value": 42}}"#).unwrap();

        assert!(config_path.exists());
        assert_eq!(config_path.file_name().unwrap(), "config.json");

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("\"test\""));
        assert!(content.contains("\"value\": 42"));
    }
}
