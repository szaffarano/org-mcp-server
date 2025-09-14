use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn create_test_org_files(temp_dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let temp_path = temp_dir.path();

    // Create a basic org file
    fs::write(
        temp_path.join("basic.org"),
        r#"* First Heading
:PROPERTIES:
:ID: heading-123
:END:
This is the first heading content.

** Sub Heading
Some sub heading content.

* Second Heading
:PROPERTIES:
:ID: heading-456
:END:
This is the second heading.
"#,
    )?;

    // Create an org file with document-level ID
    fs::write(
        temp_path.join("with_doc_id.org"),
        r#":PROPERTIES:
:ID: doc-id-789
:TITLE: Test Document
:END:

* Some Content
Regular heading content.
"#,
    )?;

    // Create an empty org file
    fs::write(temp_path.join("empty.org"), "")?;

    Ok(())
}

#[test]
fn test_list_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("list")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 3 .org files"))
        .stdout(predicate::str::contains("basic.org"))
        .stdout(predicate::str::contains("with_doc_id.org"))
        .stdout(predicate::str::contains("empty.org"));
}

#[test]
fn test_list_command_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("list")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\": 3"))
        .stdout(predicate::str::contains("\"files\""))
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("}"));
}

#[test]
fn test_list_command_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("list")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("No .org files found"));
}

#[test]
fn test_list_command_invalid_directory() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("list")
        .arg("--dir")
        .arg("/nonexistent/directory")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Directory does not exist"));
}

#[test]
fn test_read_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("read")
        .arg("basic.org")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("* First Heading"))
        .stdout(predicate::str::contains(
            "This is the first heading content",
        ));
}

#[test]
fn test_read_command_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("read")
        .arg("nonexistent.org")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .failure();
}

#[test]
fn test_outline_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("outline")
        .arg("basic.org")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("* First Heading"))
        .stdout(predicate::str::contains("** Sub Heading"))
        .stdout(predicate::str::contains("* Second Heading"));
}

#[test]
fn test_outline_command_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("outline")
        .arg("basic.org")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("}"))
        .stdout(predicate::str::contains("First Heading"));
}

#[test]
fn test_heading_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("heading")
        .arg("basic.org")
        .arg("First Heading")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("* First Heading"))
        .stdout(predicate::str::contains(
            "This is the first heading content",
        ));
}

#[test]
fn test_heading_command_nested() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("heading")
        .arg("basic.org")
        .arg("First Heading/Sub Heading")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("** Sub Heading"))
        .stdout(predicate::str::contains("Some sub heading content"));
}

#[test]
fn test_heading_command_nonexistent() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("heading")
        .arg("basic.org")
        .arg("Nonexistent Heading")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid heading"));
}

#[test]
fn test_element_by_id_command_heading() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("element-by-id")
        .arg("heading-123")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("* First Heading"))
        .stdout(predicate::str::contains(
            "This is the first heading content",
        ));
}

#[test]
fn test_element_by_id_command_document_level() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("element-by-id")
        .arg("doc-id-789")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains(":ID: doc-id-789"))
        .stdout(predicate::str::contains(":TITLE: Test Document"));
}

#[test]
fn test_element_by_id_command_nonexistent() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("element-by-id")
        .arg("nonexistent-id")
        .arg("--dir")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid element id"));
}

#[test]
fn test_init_command_basic() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("init")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success();

    // Verify the directory is accessible
    assert!(temp_dir.path().is_dir());
}

#[test]
fn test_init_command_existing_directory() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("init")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A CLI tool for org-mode functionality",
        ))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("element-by-id"))
        .stdout(predicate::str::contains("heading"));
}

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}
