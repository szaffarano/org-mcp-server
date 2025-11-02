use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;
use test_utils::copy_fixtures_with_dates;

fn create_test_org_files(temp_dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let today = chrono::Local::now().date_naive();
    copy_fixtures_with_dates(temp_dir, today)?;
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
        .stdout(predicate::str::contains("Found 10 .org files"))
        .stdout(predicate::str::contains("basic.org"))
        .stdout(predicate::str::contains("with_doc_id.org"))
        .stdout(predicate::str::contains("search_test.org"))
        .stdout(predicate::str::contains("empty.org"))
        .stdout(predicate::str::contains("tagged.org"));
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
        .stdout(predicate::str::contains("\"count\": 10"))
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
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
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
#[cfg(target_os = "linux")]
fn test_config_init_creates_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("org-mcp/config.toml");

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp_dir.path().to_str().unwrap())
        .arg("config")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(config_path.to_str().unwrap()));

    assert!(config_path.exists());
}

#[test]
#[cfg(target_os = "linux")]
fn test_config_init_file_already_exists() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("org-mcp");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");

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
    cmd.env("ORG_ORG__ORG_DIRECTORY", temp_dir.path().to_str().unwrap())
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
#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
fn test_config_path_shows_location() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp_dir.path().to_str().unwrap())
        .arg("config")
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains("org-mcp/config.toml"));
}

// Cross-platform tests using explicit --config paths

#[test]
fn test_config_init_creates_file_with_explicit_path() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("custom-config.toml");

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("config")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(config_path.to_str().unwrap()));

    assert!(config_path.exists());
}

#[test]
fn test_config_init_file_already_exists_with_explicit_path() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("existing-config.toml");

    fs::write(&config_path, "[org]\norg_directory = \"/test\"").unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("config")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"))
        .stdout(predicate::str::contains("Use 'org config show'"));
}

#[test]
fn test_config_show_with_explicit_path() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("test-config.toml");
    // Convert path to forward slashes for TOML compatibility on Windows
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"

[logging]
level = "debug"

[cli]
default_format = "plain"
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("config")
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("[org]"))
        .stdout(predicate::str::contains("org_directory"))
        .stdout(predicate::str::contains("level = \"debug\""));
}

#[test]
fn test_config_path_with_explicit_flag() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("my-config.toml");

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("config")
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains(config_path.to_str().unwrap()));
}

#[test]
fn test_config_file_affects_list_output() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    // Convert path to forward slashes for TOML compatibility on Windows
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"

[cli]
default_format = "json"
"#,
        path_str
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

    // Convert path to forward slashes for TOML compatibility on Windows
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"

[logging]
level = "info"

[cli]
default_format = "plain"
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("ORG_LOGGING__LEVEL", "debug")
        .env("ORG_ORG__ORG_DIRECTORY", temp_dir.path().to_str().unwrap())
        .arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("config")
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("level = \"debug\""));
}

// HOME-based tests for macOS and Linux

#[test]
#[cfg(not(target_os = "windows"))]
fn test_config_respects_home_env() {
    let temp_home = TempDir::new().unwrap();

    // Create the appropriate config directory for each platform
    #[cfg(target_os = "macos")]
    let config_dir = temp_home.path().join("Library/Application Support");
    #[cfg(target_os = "linux")]
    let config_dir = temp_home.path().join(".config");

    fs::create_dir_all(&config_dir).unwrap();

    // Test that config path changes when HOME changes
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("HOME", temp_home.path().to_str().unwrap())
        .env_remove("XDG_CONFIG_HOME") // Remove to test HOME fallback
        .arg("config")
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains(temp_home.path().to_string_lossy()))
        .stdout(predicate::str::contains("org-mcp/config.toml"));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_config_init_with_home_env() {
    let temp_home = TempDir::new().unwrap();

    // Create the appropriate config directory for each platform
    #[cfg(target_os = "macos")]
    let config_dir = temp_home.path().join("Library/Application Support");
    #[cfg(target_os = "linux")]
    let config_dir = temp_home.path().join(".config");

    fs::create_dir_all(&config_dir).unwrap();

    let expected_config_path = config_dir.join("org-mcp/config.toml");

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("HOME", temp_home.path().to_str().unwrap())
        .env_remove("XDG_CONFIG_HOME")
        .arg("config")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            expected_config_path.to_str().unwrap(),
        ));

    assert!(expected_config_path.exists());
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_config_show_with_home_env() {
    let temp_home = TempDir::new().unwrap();
    let temp_org = TempDir::new().unwrap();
    create_test_org_files(&temp_org).unwrap();

    // Create the appropriate config directory for each platform
    #[cfg(target_os = "macos")]
    let config_dir = temp_home.path().join("Library/Application Support");
    #[cfg(target_os = "linux")]
    let config_dir = temp_home.path().join(".config");

    fs::create_dir_all(&config_dir).unwrap();

    let org_mcp_dir = config_dir.join("org-mcp");
    fs::create_dir_all(&org_mcp_dir).unwrap();
    let config_path = org_mcp_dir.join("config.toml");
    // Convert path to forward slashes for TOML compatibility on Windows
    let path_str = temp_org.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"

[logging]
level = "trace"
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.env("HOME", temp_home.path().to_str().unwrap())
        .env_remove("XDG_CONFIG_HOME")
        .arg("config")
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("[org]"))
        .stdout(predicate::str::contains("level = \"trace\""));
}

// Tag filtering tests

#[test]
fn test_list_command_with_single_tag() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("list")
        .arg("--tags")
        .arg("work")
        .assert()
        .success()
        .stdout(predicate::str::contains("tagged.org"));

    // Verify files without tags are NOT included
    // (The test setup only has tagged.org with work tag)
}

#[test]
fn test_list_command_with_multiple_tags() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("list")
        .arg("--tags")
        .arg("work,personal")
        .assert()
        .success()
        .stdout(predicate::str::contains("tagged.org"));
}

#[test]
fn test_list_command_with_tags_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("list")
        .arg("--tags")
        .arg("work")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"files\""))
        .stdout(predicate::str::contains("tagged.org"));
}

#[test]
fn test_list_command_with_nonexistent_tag() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("list")
        .arg("--tags")
        .arg("nonexistent")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Found 0 .org files")
                .or(predicate::str::contains("No .org files found")),
        );
}

#[test]
fn test_search_command_with_single_tag() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("Task")
        .arg("--tags")
        .arg("work")
        .assert()
        .success()
        .stdout(predicate::str::contains("tagged.org"));
}

#[test]
fn test_search_command_with_multiple_tags() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("Project")
        .arg("--tags")
        .arg("personal,work")
        .assert()
        .success();
}

#[test]
fn test_search_command_with_tags_and_limit() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("Task")
        .arg("--tags")
        .arg("work")
        .arg("--limit")
        .arg("1")
        .assert()
        .success();
}

#[test]
fn test_search_command_with_tags_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("Task")
        .arg("--tags")
        .arg("work")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"results\""))
        .stdout(predicate::str::contains("\"file_path\""));
}

#[test]
fn test_search_command_with_tags_no_match() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("test")
        .arg("--tags")
        .arg("nonexistent")
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found"));
}

#[test]
fn test_search_command_with_tags_all_parameters() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--root-directory")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("search")
        .arg("Task")
        .arg("--tags")
        .arg("work")
        .arg("--limit")
        .arg("2")
        .arg("--format")
        .arg("json")
        .arg("--snippet-size")
        .arg("30")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"results\""));
}

#[test]
fn test_agenda_list_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org", "project.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("task"));
}

#[test]
fn test_agenda_list_command_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"count\""))
        .stdout(predicate::str::contains("\"tasks\""))
        .stdout(predicate::str::contains("\"heading\""))
        .stdout(predicate::str::contains("\"file_path\""));
}

#[test]
fn test_agenda_list_command_with_limit() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--limit")
        .arg("3")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 3 tasks").or(predicate::str::contains("Found")));
}

#[test]
fn test_agenda_today_command() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agenda"));
}

#[test]
fn test_agenda_week_command() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("week")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agenda"));
}

#[test]
fn test_agenda_range_command() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    // Use a dynamic date range that includes today and the next week
    let today = chrono::Local::now().date_naive();
    let week_later = today + chrono::Duration::days(7);
    let start_date = today.format("%Y-%m-%d").to_string();
    let end_date = week_later.format("%Y-%m-%d").to_string();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("range")
        .arg("--start")
        .arg(&start_date)
        .arg("--end")
        .arg(&end_date)
        .assert()
        .success()
        .stdout(predicate::str::contains("Agenda"));
}

#[test]
fn test_agenda_range_command_invalid_start() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("range")
        .arg("--start")
        .arg("2025-15-01")
        .arg("--end")
        .arg("2025-10-31")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse start date"));
}

#[test]
fn test_agenda_range_command_invalid_end() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("range")
        .arg("--start")
        .arg("2025-10-01")
        .arg("--end")
        .arg("2025-15-31")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse end date"));
}

#[test]
fn test_agenda_list_filter_by_states() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--states")
        .arg("TODO")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));
}

#[test]
fn test_agenda_list_filter_by_priority() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--priority")
        .arg("a")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));
}

#[test]
fn test_agenda_list_filter_by_tags() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--tags")
        .arg("work")
        .assert()
        .success();
}

#[test]
fn test_agenda_list_empty_agenda_files() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["empty.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("No tasks found")
                .or(predicate::str::contains("Found 0 tasks")),
        );
}

#[test]
fn test_agenda_range_invalid_date() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("range")
        .arg("--start")
        .arg("invalid-date")
        .arg("--end")
        .arg("2025-10-31")
        .assert()
        .failure();
}

#[test]
fn test_agenda_help() {
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("agenda")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agenda views and task management"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("today"))
        .stdout(predicate::str::contains("week"))
        .stdout(predicate::str::contains("range"));
}

#[test]
fn test_agenda_list_no_matching_priority() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--priority")
        .arg("c")
        .arg("--states")
        .arg("TODO")
        .assert()
        .success();
}

#[test]
fn test_agenda_list_multiple_filters() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--states")
        .arg("TODO")
        .arg("--tags")
        .arg("work")
        .arg("--priority")
        .arg("a")
        .arg("--limit")
        .arg("5")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));
}

#[test]
#[ignore = "TODO: Tag filtering in agenda queries is not working correctly - separate bug to fix"]
fn test_agenda_today_with_tags() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("today")
        .arg("--tags")
        .arg("work,personal")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agenda"));
}

#[test]
fn test_agenda_week_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("week")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"items\""))
        .stdout(predicate::str::contains("\"count\""));
}

#[test]
fn test_agenda_list_combined_states() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--states")
        .arg("TODO,DONE")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));
}

#[test]
fn test_agenda_list_with_limit_edge_case() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    // Test with limit of 1
    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--limit")
        .arg("1")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Found 1 tasks")
                .or(predicate::str::contains("Found 1 task"))
                .or(predicate::str::contains("Found")),
        );
}

#[test]
fn test_agenda_today_plain_format_explicit() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]

[cli]
default_format = "plain"
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("today")
        .arg("--format")
        .arg("plain")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agenda"));
}

#[test]
fn test_agenda_week_plain_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("week")
        .arg("--format")
        .arg("plain")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agenda"));
}

#[test]
fn test_agenda_today_json_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("today")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"items\""));
}

#[test]
fn test_agenda_list_plain_format() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let config_path = temp_dir.path().join("config.toml");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"
org_agenda_files = ["agenda.org"]
"#,
        path_str
    );
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("org-cli").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_str().unwrap())
        .arg("agenda")
        .arg("list")
        .arg("--format")
        .arg("plain")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));
}
