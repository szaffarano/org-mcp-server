//! Shared test utilities and fixtures for org-mcp-server integration tests.
//!
//! This crate provides common test fixtures and helper functions to set up
//! test environments for org-cli and org-mcp-server integration tests.

pub mod config;

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Path to the fixtures directory relative to this crate's root
const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");

/// Copies all test fixtures to a temporary directory.
///
/// This function copies the entire fixtures directory structure, including
/// subdirectories, to the provided temporary directory.
///
/// # Arguments
/// * `temp_dir` - The temporary directory to copy fixtures into
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error
///
/// # Example
/// ```no_run
/// use tempfile::TempDir;
/// use test_utils::copy_fixtures_to_temp;
///
/// let temp_dir = TempDir::new()?;
/// copy_fixtures_to_temp(&temp_dir)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn copy_fixtures_to_temp(temp_dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
    copy_dir_recursive(Path::new(FIXTURES_DIR), temp_dir.path())?;
    Ok(())
}

/// Copies a directory and all its contents recursively.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Creates a temporary directory with all test fixtures.
///
/// This is a convenience function that combines `TempDir::new()` and
/// `copy_fixtures_to_temp()`. It's particularly useful for MCP server tests
/// that need a fresh temporary directory with fixtures.
///
/// # Returns
/// * `Result<TempDir, Box<dyn std::error::Error>>` - Temporary directory with fixtures
///
/// # Example
/// ```no_run
/// use test_utils::setup_test_org_files;
///
/// let temp_dir = setup_test_org_files()?;
/// // Use temp_dir.path() to access the fixtures
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn setup_test_org_files() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    copy_fixtures_to_temp(&temp_dir)?;
    Ok(temp_dir)
}

/// Copies specific fixture files to a temporary directory.
///
/// # Arguments
/// * `temp_dir` - The temporary directory to copy fixtures into
/// * `files` - List of fixture filenames to copy (relative to fixtures directory)
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error
///
/// # Example
/// ```no_run
/// use tempfile::TempDir;
/// use test_utils::copy_specific_fixtures;
///
/// let temp_dir = TempDir::new()?;
/// copy_specific_fixtures(&temp_dir, &["basic.org", "tagged.org"])?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn copy_specific_fixtures(
    temp_dir: &TempDir,
    files: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let fixtures_path = Path::new(FIXTURES_DIR);

    for file in files {
        let src = fixtures_path.join(file);
        let dst = temp_dir.path().join(file);

        // Create parent directory if needed
        if let Some(parent) = dst.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        fs::copy(&src, &dst)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_fixtures_to_temp() {
        let temp_dir = TempDir::new().unwrap();
        copy_fixtures_to_temp(&temp_dir).unwrap();

        // Verify some expected files exist
        assert!(temp_dir.path().join("basic.org").exists());
        assert!(temp_dir.path().join("tagged.org").exists());
        assert!(temp_dir.path().join("notes.org").exists());
        assert!(temp_dir.path().join("archive/old_notes.org").exists());
    }

    #[test]
    fn test_setup_test_org_files() {
        let temp_dir = setup_test_org_files().unwrap();

        // Verify fixture files exist
        assert!(temp_dir.path().join("basic.org").exists());
        assert!(temp_dir.path().join("project.org").exists());
        assert!(temp_dir.path().join("research.org").exists());
    }

    #[test]
    fn test_copy_specific_fixtures() {
        let temp_dir = TempDir::new().unwrap();
        copy_specific_fixtures(&temp_dir, &["basic.org", "tagged.org"]).unwrap();

        // Verify only specified files exist
        assert!(temp_dir.path().join("basic.org").exists());
        assert!(temp_dir.path().join("tagged.org").exists());
        assert!(!temp_dir.path().join("notes.org").exists());
    }
}
