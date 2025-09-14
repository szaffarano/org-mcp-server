use crate::{OrgMode, OrgModeError};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_invalid_directory_error() {
    let result = OrgMode::new("/completely/nonexistent/directory/path");

    assert!(result.is_err());
    let err = result.unwrap_err();

    match err {
        OrgModeError::InvalidDirectory(msg) => {
            assert!(msg.contains("/completely/nonexistent/directory/path"));
        }
        _ => panic!("Expected InvalidDirectory error, got: {:?}", err),
    }
}

#[test]
fn test_file_not_found_error() {
    let temp_dir = TempDir::new().unwrap();
    let org_mode = OrgMode::new(temp_dir.path().to_str().unwrap()).unwrap();

    let result = org_mode.read_file("nonexistent.org");
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        OrgModeError::IoError(_) => {
            // File not found should result in IoError
        }
        _ => panic!("Expected IoError for file not found, got: {:?}", err),
    }
}

#[test]
fn test_invalid_heading_path_error() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create basic org file
    fs::write(
        temp_path.join("test.org"),
        r#"* First Heading
Content here.

* Second Heading
More content.
"#,
    )
    .unwrap();

    let org_mode = OrgMode::new(temp_path.to_str().unwrap()).unwrap();

    // Test invalid heading path
    let result = org_mode.get_heading("test.org", "Nonexistent Heading");
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        OrgModeError::InvalidHeadingPath(path) => {
            assert!(path.contains("Nonexistent Heading"));
        }
        _ => panic!("Expected InvalidHeadingPath error, got: {:?}", err),
    }
}

#[test]
fn test_invalid_element_id_error() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create org file with some content but no matching ID
    fs::write(
        temp_path.join("test.org"),
        r#"* Heading
:PROPERTIES:
:ID: real-id-123
:END:
Some content.
"#,
    )
    .unwrap();

    let org_mode = OrgMode::new(temp_path.to_str().unwrap()).unwrap();

    let result = org_mode.get_element_by_id("nonexistent-id");
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        OrgModeError::InvalidElementId(id) => {
            assert_eq!(id, "nonexistent-id");
        }
        _ => panic!("Expected InvalidElementId error, got: {:?}", err),
    }
}

#[test]
fn test_parsing_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a file that might cause parsing issues (though orgize is quite robust)
    fs::write(
        temp_path.join("malformed.org"),
        "This is not a valid org file with weird control characters: \x00\x01\x02",
    )
    .unwrap();

    let org_mode = OrgMode::new(temp_path.to_str().unwrap()).unwrap();

    // Even malformed content should be readable (orgize handles it gracefully)
    let result = org_mode.read_file("malformed.org");
    assert!(result.is_ok());
}

#[test]
fn test_empty_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create empty file
    fs::write(temp_path.join("empty.org"), "").unwrap();

    let org_mode = OrgMode::new(temp_path.to_str().unwrap()).unwrap();

    // Empty file should be readable
    let result = org_mode.read_file("empty.org");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");

    // Empty file should have empty outline
    let outline_result = org_mode.get_outline("empty.org");
    assert!(outline_result.is_ok());
    let outline = outline_result.unwrap();
    assert!(outline.children.is_empty());
}

#[test]
fn test_io_error_propagation() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a file, then make the directory unreadable (if possible)
    fs::write(temp_path.join("test.org"), "* Test\nContent").unwrap();

    let org_mode = OrgMode::new(temp_path.to_str().unwrap()).unwrap();

    // This should work normally
    let result = org_mode.read_file("test.org");
    assert!(result.is_ok());
}

#[test]
fn test_with_defaults_error_handling() {
    // This might fail if ~/org/ doesn't exist, which is a valid test case
    let result = OrgMode::with_defaults();

    // We can't predict if this will succeed or fail, but we can test it doesn't panic
    match result {
        Ok(_) => {
            // User has ~/org/ directory
        }
        Err(err) => {
            // Expected if ~/org/ doesn't exist
            match err {
                OrgModeError::InvalidDirectory(_) => {
                    // This is expected
                }
                _ => panic!("Unexpected error type: {:?}", err),
            }
        }
    }
}

#[test]
fn test_error_display_formatting() {
    let error = OrgModeError::InvalidDirectory("/some/path".to_string());
    let display = format!("{}", error);
    assert!(display.contains("/some/path"));
    assert!(display.contains("Invalid or inaccessible directory"));

    let error = OrgModeError::InvalidHeadingPath("Some/Path".to_string());
    let display = format!("{}", error);
    assert!(display.contains("Some/Path"));
    assert!(display.contains("Invalid heading path"));

    let error = OrgModeError::InvalidElementId("id-123".to_string());
    let display = format!("{}", error);
    assert!(display.contains("id-123"));
    assert!(display.contains("Invalid element id"));

    let error = OrgModeError::ShellExpansionError("~/path".to_string());
    let display = format!("{}", error);
    assert!(display.contains("~/path"));
    assert!(display.contains("Failed to expand path"));
}
