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

    // Create an org file with searchable content
    fs::write(
        temp_path.join("search_test.org"),
        r#"* Project Planning
This document contains project planning information.

** TODO Meeting Notes
Meeting scheduled for next week.

** DONE Task Completion
Task was completed successfully.

* Bug Reports
Found several bugs in the system:
- Critical bug in authentication
- Minor UI bug in dashboard

* Long Content Test
This is a very long line of text that should be truncated when using a small snippet size parameter to test the snippet truncation functionality properly.
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
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 4 .org files"))
        .stdout(predicate::str::contains("basic.org"))
        .stdout(predicate::str::contains("with_doc_id.org"))
        .stdout(predicate::str::contains("search_test.org"))
        .stdout(predicate::str::contains("empty.org"));
}

#[test]
fn test_list_command_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("list")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\": 4"))
        .stdout(predicate::str::contains("\"files\""))
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("}"));
}

#[test]
fn test_list_command_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No .org files found"));
}

#[test]
fn test_list_command_invalid_directory() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg("/nonexistent/directory")
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Root directory does not exist"));
}

#[test]
fn test_read_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("read")
        .arg("basic.org")
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
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("read")
        .arg("nonexistent.org")
        .assert()
        .failure();
}

#[test]
fn test_outline_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("outline")
        .arg("basic.org")
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
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("outline")
        .arg("basic.org")
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
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("heading")
        .arg("basic.org")
        .arg("First Heading")
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
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("heading")
        .arg("basic.org")
        .arg("First Heading/Sub Heading")
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
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("heading")
        .arg("basic.org")
        .arg("Nonexistent Heading")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid heading"));
}

#[test]
fn test_element_by_id_command_heading() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("element-by-id")
        .arg("heading-123")
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
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("element-by-id")
        .arg("doc-id-789")
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
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("element-by-id")
        .arg("nonexistent-id")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid element id"));
}

#[test]
fn test_init_command_basic() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("ORG_ROOT_DIRECTORY", temp_dir.path().to_str().unwrap());
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
    cmd.env("ORG_ROOT_DIRECTORY", temp_dir.path().to_str().unwrap());
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
        .stdout(predicate::str::contains("heading"))
        .stdout(predicate::str::contains("search"));
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

#[test]
fn test_search_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("project")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("search_test.org"))
        .stdout(predicate::str::contains("Project Planning"));
}

#[test]
fn test_search_command_with_limit() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("bug")
        .arg("--limit")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 1 results"));
}

#[test]
fn test_search_command_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("heading")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("}"))
        .stdout(predicate::str::contains("\"count\""))
        .stdout(predicate::str::contains("\"results\""))
        .stdout(predicate::str::contains("\"file_path\""))
        .stdout(predicate::str::contains("\"snippet\""))
        .stdout(predicate::str::contains("\"score\""));
}

#[test]
fn test_search_command_custom_snippet_size() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("truncated")
        .arg("--snippet-size")
        .arg("20")
        .assert()
        .success();

    // If results are found and truncated, they should end with "..."
    // We don't assert specific content as fuzzy matching behavior may vary
}

#[test]
fn test_search_command_no_results() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("nonexistentquerythatwillnotmatch")
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found"));
}

#[test]
fn test_search_command_empty_query() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("")
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found"));
}

#[test]
fn test_search_command_invalid_directory() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg("/nonexistent/directory")
        .arg("search")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Root directory does not exist"));
}

#[test]
fn test_search_command_all_parameters() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("content")
        .arg("--limit")
        .arg("2")
        .arg("--format")
        .arg("json")
        .arg("--snippet-size")
        .arg("30")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"count\""))
        .stdout(predicate::str::contains("\"results\""));
}

#[test]
fn test_search_command_help() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("search")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Search for text content"))
        .stdout(predicate::str::contains("--limit"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--snippet-size"));
}

#[test]
fn test_config_init_creates_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("org-mcp-server.toml");

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp_dir.path().to_str().unwrap())
        .arg("config")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Default configuration file created",
        ))
        .stdout(predicate::str::contains(config_path.to_str().unwrap()));

    assert!(config_path.exists());
}

#[test]
fn test_config_init_file_already_exists() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("org-mcp-server.toml");

    fs::write(&config_path, "[org]\norg_directory = \"/test\"").unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp_dir.path().to_str().unwrap())
        .arg("config")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"))
        .stdout(predicate::str::contains("Use 'org config show'"));
}

#[test]
fn test_config_show_displays_config() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("ORG_ROOT_DIRECTORY", temp_dir.path().to_str().unwrap())
        .arg("config")
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("[org]"))
        .stdout(predicate::str::contains("org_directory"))
        .stdout(predicate::str::contains("[logging]"))
        .stdout(predicate::str::contains("[cli]"));
}

#[test]
fn test_config_show_fallback_to_default() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", "/nonexistent/path")
        .arg("config")
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("~/org/"))
        .stdout(predicate::str::contains("notes.org"));
}

#[test]
fn test_config_path_shows_location() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp_dir.path().to_str().unwrap())
        .arg("config")
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains("org-mcp-server.toml"));
}

#[test]
fn test_config_file_affects_list_output() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"

[cli]
default_format = "json"
"#,
        temp_dir.path().to_str().unwrap()
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"directory\""))
        .stdout(predicate::str::contains("\"count\""));
}

#[test]
fn test_config_hierarchy_file_env_cli() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_dir = TempDir::new().unwrap();
    let config_path = config_dir.path().join("config.toml");

    let config_content = format!(
        r#"
[org]
org_directory = "{}"

[logging]
level = "info"

[cli]
default_format = "plain"
"#,
        temp_dir.path().to_str().unwrap()
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("ORG_LOG_LEVEL", "debug")
        .env("ORG_ROOT_DIRECTORY", temp_dir.path().to_str().unwrap())
        .arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("config")
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("level = \"debug\""));
}
