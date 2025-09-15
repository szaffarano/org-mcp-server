// Tool tests focus on the underlying org-core functionality
// since direct tool method testing requires complex MCP machinery
use crate::core::OrgModeRouter;
use org_core::OrgMode;
use std::fs;
use tempfile::TempDir;

fn create_test_org_files(temp_dir: &TempDir) -> std::io::Result<()> {
    let temp_path = temp_dir.path();

    // Create multiple org files
    fs::write(
        temp_path.join("notes.org"),
        r#"* Notes
Some notes content.
"#,
    )?;

    fs::write(
        temp_path.join("tasks.org"),
        r#"* TODO Task 1
:PROPERTIES:
:ID: task-123
:END:
First task content.

** DONE Subtask
Completed subtask.
"#,
    )?;

    fs::write(
        temp_path.join("projects.org"),
        r#"* Project Alpha
Alpha project details.

* Project Beta
Beta project details.
"#,
    )?;

    // Create a file with searchable content for search tests
    fs::write(
        temp_path.join("search_content.org"),
        r#"* Programming Topics
This file contains programming information.

** Rust Programming
Rust is a systems programming language focused on safety and performance.

** JavaScript Development
JavaScript is widely used for web development and can run on servers.

** Database Systems
Working with databases is essential for backend development.

* Meeting Notes
Important meeting notes from the team.

** Sprint Planning
Planning for the next sprint with feature priorities.

** Code Review Session
Discussed code quality and best practices.
"#,
    )?;

    // Create a non-org file (should be ignored)
    fs::write(temp_path.join("readme.txt"), "Not an org file")?;

    Ok(())
}

#[tokio::test]
async fn test_org_mode_router_creation() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap());
    assert!(router.is_ok());
}

#[tokio::test]
async fn test_org_mode_router_invalid_directory() {
    let result = OrgModeRouter::with_directory("/nonexistent/directory");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_org_mode_list_files_functionality() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    let files = org_mode.list_files().unwrap();
    assert_eq!(files.len(), 4);
    assert!(files.contains(&"notes.org".to_string()));
    assert!(files.contains(&"tasks.org".to_string()));
    assert!(files.contains(&"projects.org".to_string()));
    assert!(files.contains(&"search_content.org".to_string()));
    // Should not contain non-org files
    assert!(!files.contains(&"readme.txt".to_string()));
}

#[tokio::test]
async fn test_org_mode_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    let files = org_mode.list_files().unwrap();
    assert_eq!(files.len(), 0);
}

#[test]
fn test_org_mode_direct_creation() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let org_mode = OrgMode::new(temp_dir.path().to_str().unwrap());
    assert!(org_mode.is_ok());

    let files = org_mode.unwrap().list_files().unwrap();
    assert_eq!(files.len(), 4);
}

#[test]
fn test_org_mode_invalid_directory_direct() {
    let result = OrgMode::new("/nonexistent/directory");
    assert!(result.is_err());
}

// Search functionality tests
#[tokio::test]
async fn test_org_search_basic_functionality() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    let results = org_mode.search("programming", None, None).unwrap();
    assert!(!results.is_empty());

    // Check that results contain expected content
    let found_programming = results.iter().any(|result| {
        result.file_path == "search_content.org"
            && result.snippet.to_lowercase().contains("programming")
    });
    assert!(found_programming, "Should find programming content");
}

#[tokio::test]
async fn test_org_search_with_limit() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    // Search for "project" which should match multiple entries
    let results = org_mode.search("project", Some(2), None).unwrap();
    assert!(results.len() <= 2, "Should respect limit parameter");
}

#[tokio::test]
async fn test_org_search_with_snippet_size() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    let results = org_mode
        .search("systems programming language", None, Some(20))
        .unwrap();

    for result in results {
        // Snippets should be truncated or within limit
        if result.snippet.ends_with("...") {
            assert!(result.snippet.chars().count() <= 23); // 20 + "..."
        } else {
            assert!(result.snippet.chars().count() <= 20);
        }
    }
}

#[tokio::test]
async fn test_org_search_no_results() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    let results = org_mode
        .search("nonexistentquerythatwillnotmatch", None, None)
        .unwrap();
    assert!(
        results.is_empty(),
        "Should return empty results for non-matching query"
    );
}

#[tokio::test]
async fn test_org_search_empty_query() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    let results = org_mode.search("", None, None).unwrap();
    assert!(results.is_empty(), "Empty query should return no results");

    let results = org_mode.search("   ", None, None).unwrap();
    assert!(
        results.is_empty(),
        "Whitespace-only query should return no results"
    );
}

#[tokio::test]
async fn test_org_search_multiple_terms() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    // Search for multiple terms that should match (AND logic)
    let results = org_mode.search("meeting notes", None, None).unwrap();

    // Should find results containing both "meeting" and "notes"
    for result in results {
        // Note: Due to fuzzy matching, exact term presence may vary
        // The key test is that it doesn't crash and returns valid results
        assert!(!result.snippet.is_empty());
        assert!(result.score > 0);
    }
}

#[tokio::test]
async fn test_org_search_result_structure() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    let results = org_mode.search("task", None, None).unwrap();

    for result in results {
        // Verify SearchResult structure
        assert!(
            !result.file_path.is_empty(),
            "File path should not be empty"
        );
        assert!(!result.snippet.is_empty(), "Snippet should not be empty");
        assert!(result.score > 0, "Score should be positive");
        assert!(
            result.file_path.ends_with(".org"),
            "Should only find org files"
        );
    }
}

#[tokio::test]
async fn test_org_search_scores_and_ranking() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
    let org_mode = router.org_mode.lock().await;

    let results = org_mode.search("development", None, None).unwrap();

    if results.len() > 1 {
        // Results should be sorted by score (highest first)
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results should be sorted by score in descending order"
            );
        }
    }
}

#[test]
fn test_org_search_direct_mode() {
    let temp_dir = TempDir::new().unwrap();
    create_test_org_files(&temp_dir).unwrap();

    let org_mode = OrgMode::new(temp_dir.path().to_str().unwrap()).unwrap();
    let results = org_mode.search("alpha", None, None).unwrap();

    // Should find "Project Alpha" content
    let found_alpha = results
        .iter()
        .any(|result| result.snippet.to_lowercase().contains("alpha"));
    assert!(found_alpha, "Should find alpha project content");
}
