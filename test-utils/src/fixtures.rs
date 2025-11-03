use chrono::NaiveDate;
use std::path::Path;
use std::{error, fs};
use tempfile::TempDir;

use crate::dates;

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
/// use test_utils::fixtures::copy_fixtures_to_temp;
///
/// let temp_dir = TempDir::new()?;
/// copy_fixtures_to_temp(&temp_dir)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn copy_fixtures_to_temp(temp_dir: &TempDir) -> Result<(), Box<dyn error::Error>> {
    copy_dir_recursive(Path::new(FIXTURES_DIR), temp_dir.path())?;
    Ok(())
}

/// Copies a directory and all its contents recursively.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn error::Error>> {
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
/// `copy_fixtures_to_temp()`.
/// # Returns
/// * `Result<TempDir, Box<dyn std::error::Error>>` - Temporary directory with fixtures
///
/// # Example
/// ```no_run
/// use test_utils::fixtures::setup_test_org_files;
///
/// let temp_dir = setup_test_org_files()?;
/// // Use temp_dir.path() to access the fixtures
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn setup_test_org_files() -> Result<TempDir, Box<dyn error::Error>> {
    let temp_dir = TempDir::new()?;
    copy_fixtures_to_temp(&temp_dir)?;
    Ok(temp_dir)
}

/// Creates a temporary directory with all test fixtures with placeholder repacement.
///
/// This is a convenience function that combines `TempDir::new()` and
/// `copy_fixtures_with_dates()`.
/// # Returns
/// * `Result<TempDir, Box<dyn std::error::Error>>` - Temporary directory with fixtures
///
/// # Example
/// ```no_run
/// use test_utils::fixtures::setup_test_org_files_with_dates;
///
/// let temp_dir = setup_test_org_files_with_dates()?;
/// // Use temp_dir.path() to access the fixtures
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn setup_test_org_files_with_dates() -> Result<TempDir, Box<dyn error::Error>> {
    let temp_dir = TempDir::new()?;
    copy_fixtures_with_dates(&temp_dir, chrono::Local::now().date_naive())?;
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
/// use test_utils::fixtures::copy_specific_fixtures;
///
/// let temp_dir = TempDir::new()?;
/// copy_specific_fixtures(&temp_dir, &["basic.org", "tagged.org"])?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn copy_specific_fixtures(
    temp_dir: &TempDir,
    files: &[&str],
) -> Result<(), Box<dyn error::Error>> {
    let fixtures_path = Path::new(FIXTURES_DIR);

    for file in files {
        let src = fixtures_path.join(file);
        let dst = temp_dir.path().join(file);

        if let Some(parent) = dst.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        fs::copy(&src, &dst)?;
    }

    Ok(())
}

/// Copies all test fixtures to a temporary directory with date placeholder replacement.
///
/// This function copies the entire fixtures directory structure, replacing date placeholders
/// like `@TODAY@`, `@TODAY+N@`, etc. with actual dates relative to the provided base date.
///
/// # Arguments
/// * `temp_dir` - The temporary directory to copy fixtures into
/// * `base_date` - The base date to use for placeholder replacement
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error
///
/// # Example
/// ```no_run
/// use tempfile::TempDir;
/// use test_utils::fixtures::copy_fixtures_with_dates;
/// use chrono::Local;
///
/// let temp_dir = TempDir::new()?;
/// let today = Local::now().date_naive();
/// copy_fixtures_with_dates(&temp_dir, today)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn copy_fixtures_with_dates(
    temp_dir: &TempDir,
    base_date: NaiveDate,
) -> Result<(), Box<dyn error::Error>> {
    copy_dir_with_date_replacement(Path::new(FIXTURES_DIR), temp_dir.path(), base_date)?;
    Ok(())
}

/// Copies a directory and all its contents recursively, replacing date placeholders in .org files.
fn copy_dir_with_date_replacement(
    src: &Path,
    dst: &Path,
    base_date: NaiveDate,
) -> Result<(), Box<dyn error::Error>> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_with_date_replacement(&src_path, &dst_path, base_date)?;
        } else if src_path.extension().and_then(|s| s.to_str()) == Some("org") {
            // Read .org file, replace dates, and write to destination
            let content = fs::read_to_string(&src_path)?;
            let modified_content = dates::replace_dates_in_content(&content, base_date);
            fs::write(&dst_path, modified_content)?;
        } else {
            // Copy non-.org files directly
            fs::copy(&src_path, &dst_path)?;
        }
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

        assert!(temp_dir.path().join("basic.org").exists());
        assert!(temp_dir.path().join("tagged.org").exists());
        assert!(temp_dir.path().join("notes.org").exists());
        assert!(temp_dir.path().join("archive/old_notes.org").exists());
    }

    #[test]
    fn test_setup_test_org_files() {
        let temp_dir = setup_test_org_files().unwrap();

        assert!(temp_dir.path().join("basic.org").exists());
        assert!(temp_dir.path().join("project.org").exists());
        assert!(temp_dir.path().join("research.org").exists());
    }

    #[test]
    fn test_copy_specific_fixtures() {
        let temp_dir = TempDir::new().unwrap();
        copy_specific_fixtures(&temp_dir, &["tagged.org"]).unwrap();

        assert!(temp_dir.path().join("tagged.org").exists());
        assert!(!temp_dir.path().join("notes.org").exists());
    }
}
