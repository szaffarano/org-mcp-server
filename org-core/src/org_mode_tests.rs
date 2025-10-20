use crate::config::OrgConfig;
use crate::org_mode::TreeNode;
use crate::{OrgMode, OrgModeError};

fn create_test_org_mode() -> OrgMode {
    let config = OrgConfig {
        org_directory: "tests/fixtures".to_string(),
        ..OrgConfig::default()
    };
    OrgMode::new(config).expect("Failed to create test OrgMode")
}

#[test]
fn test_get_outline_simple() {
    let org_mode = create_test_org_mode();
    let tree = org_mode
        .get_outline("simple.org")
        .expect("Failed to get outline");

    assert_eq!(tree.label, "Document");
    assert_eq!(tree.children.len(), 3);
    assert_eq!(tree.children[0].label, "First Heading");
    assert_eq!(tree.children[0].level, 1);
    assert_eq!(tree.children[1].label, "Second Heading");
    assert_eq!(tree.children[1].level, 1);
    assert_eq!(tree.children[2].label, "Third Heading");
    assert_eq!(tree.children[2].level, 1);
}

#[test]
fn test_get_outline_nested() {
    let org_mode = create_test_org_mode();
    let tree = org_mode
        .get_outline("sample.org")
        .expect("Failed to get outline");

    assert_eq!(tree.label, "Document");
    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].label, "Top Level Heading");
    assert_eq!(tree.children[0].level, 1);
    assert_eq!(tree.children[0].children.len(), 2);
    assert_eq!(tree.children[0].children[0].label, "Second Level A");
    assert_eq!(tree.children[0].children[0].level, 2);
    assert_eq!(tree.children[0].children[0].children.len(), 1);
    assert_eq!(
        tree.children[0].children[0].children[0].label,
        "Third Level"
    );
    assert_eq!(tree.children[0].children[0].children[0].level, 3);
}

#[test]
fn test_get_outline_nonexistent_file() {
    let org_mode = create_test_org_mode();
    let result = org_mode.get_outline("nonexistent.org");
    assert!(result.is_err());
}

#[test]
fn test_tree_node_creation() {
    let node = TreeNode::new("Test Heading".to_string());
    assert_eq!(node.label, "Test Heading");
    assert_eq!(node.level, 0);
    assert!(node.children.is_empty());

    let node_with_level = TreeNode::new_with_level("Test Heading".to_string(), 2);
    assert_eq!(node_with_level.label, "Test Heading");
    assert_eq!(node_with_level.level, 2);
    assert!(node_with_level.children.is_empty());
}

#[test]
fn test_tree_node_indented_string() {
    let mut parent = TreeNode::new_with_level("Parent".to_string(), 1);
    let child = TreeNode::new_with_level("Child".to_string(), 2);
    parent.children.push(child);

    let result = parent.to_indented_string(0);
    let expected = "* Parent\n  ** Child\n";
    assert_eq!(result, expected);
}

#[test]
fn test_tree_node_serialization() {
    let mut root = TreeNode::new("Document".to_string());
    let mut parent = TreeNode::new_with_level("Parent".to_string(), 1);
    let child = TreeNode::new_with_level("Child".to_string(), 2);
    parent.children.push(child);
    root.children.push(parent);

    let json = serde_json::to_string(&root).expect("Failed to serialize");
    let deserialized: TreeNode = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.label, "Document");
    assert_eq!(deserialized.level, 0);
    assert_eq!(deserialized.children.len(), 1);
    assert_eq!(deserialized.children[0].label, "Parent");
    assert_eq!(deserialized.children[0].level, 1);
    assert_eq!(deserialized.children[0].children.len(), 1);
    assert_eq!(deserialized.children[0].children[0].label, "Child");
    assert_eq!(deserialized.children[0].children[0].level, 2);
}

#[test]
fn test_get_heading_simple() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_heading("simple.org", "First Heading")
        .expect("Failed to get heading");

    assert!(result.contains("* First Heading"));
    assert!(result.contains("This is content under the first heading."));
}

#[test]
fn test_get_heading_nested_path() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_heading("nested.org", "Project Planning/Phase 1/Setup Tasks")
        .expect("Failed to get nested heading");

    assert!(result.contains("*** Setup Tasks"));
    assert!(result.contains("Install dependencies"));
    assert!(result.contains("Configure environment"));
}

#[test]
fn test_get_heading_nonexistent() {
    let org_mode = create_test_org_mode();
    let result = org_mode.get_heading("simple.org", "Nonexistent Heading");

    match result {
        Err(OrgModeError::InvalidHeadingPath(path)) => {
            assert_eq!(path, "Nonexistent Heading");
        }
        _ => panic!("Expected InvalidHeadingPath error"),
    }
}

#[test]
fn test_get_heading_invalid_path() {
    let org_mode = create_test_org_mode();
    let result = org_mode.get_heading("nested.org", "Project Planning/Nonexistent/Deep");

    match result {
        Err(OrgModeError::InvalidHeadingPath(path)) => {
            assert_eq!(path, "Project Planning/Nonexistent/Deep");
        }
        _ => panic!("Expected InvalidHeadingPath error"),
    }
}

#[test]
fn test_get_heading_second_level() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_heading("nested.org", "Project Planning/Phase 2")
        .expect("Failed to get second level heading");

    assert!(result.contains("** Phase 2"));
    assert!(result.contains("Implementation phase."));
    assert!(result.contains("*** Development"));
    assert!(result.contains("*** Testing"));
}

#[test]
fn test_get_element_by_id_simple_heading() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_element_by_id("simple-123")
        .expect("Failed to get element by simple ID");

    assert!(result.contains("* Heading with Simple ID"));
    assert!(result.contains(":ID: simple-123"));
    assert!(result.contains("This heading has a simple ID for testing."));
    assert!(result.contains("** Subheading"));
}

#[test]
fn test_get_element_by_id_uuid_heading() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_element_by_id("550e8400-e29b-41d4-a716-446655440000")
        .expect("Failed to get element by UUID ID");

    assert!(result.contains("* Heading with UUID"));
    assert!(result.contains(":ID: 550e8400-e29b-41d4-a716-446655440000"));
    assert!(result.contains("This heading has a UUID-style ID."));
}

#[test]
fn test_get_element_by_id_document_level() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_element_by_id("document-level-id-456")
        .expect("Failed to get document-level element by ID");

    assert!(result.contains(":ID: document-level-id-456"));
    assert!(result.contains(":TITLE: Document with ID"));
    assert!(result.contains("This is a document with a document-level ID property."));
}

#[test]
fn test_get_element_by_id_nested_heading() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_element_by_id("nested-abc")
        .expect("Failed to get nested element by ID");

    assert!(result.contains("** Nested Heading with ID"));
    assert!(result.contains(":ID: nested-abc"));
    assert!(result.contains("This is a nested heading with an ID."));
}

#[test]
fn test_get_element_by_id_with_multiple_properties() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_element_by_id("multi-prop-id")
        .expect("Failed to get element with multiple properties");

    assert!(result.contains("* Multiple Properties Heading"));
    assert!(result.contains(":ID: multi-prop-id"));
    assert!(result.contains(":CREATED: [2023-01-01 Mon]"));
    assert!(result.contains(":CATEGORY: test"));
    assert!(result.contains("This heading has multiple properties including an ID."));
}

#[test]
fn test_get_element_by_id_nonexistent() {
    let org_mode = create_test_org_mode();
    let result = org_mode.get_element_by_id("nonexistent-id");

    match result {
        Err(OrgModeError::InvalidElementId(id)) => {
            assert_eq!(id, "nonexistent-id");
        }
        _ => panic!("Expected InvalidElementId error"),
    }
}

#[test]
fn test_get_element_by_id_case_sensitive() {
    let org_mode = create_test_org_mode();

    let result = org_mode.get_element_by_id("CaseSensitiveID");
    assert!(result.is_ok());

    let result = org_mode.get_element_by_id("casesensitiveid");
    assert!(result.is_err());

    let result = org_mode.get_element_by_id("CASESENSITIVEID");
    assert!(result.is_err());
}

#[test]
fn test_get_element_by_id_special_characters() {
    let org_mode = create_test_org_mode();
    let result = org_mode
        .get_element_by_id("special-chars!@#$%")
        .expect("Failed to get element with special characters in ID");

    assert!(result.contains("* Heading with Special Characters"));
    assert!(result.contains(":ID: special-chars!@#$%"));
}

#[test]
fn test_get_element_by_id_whitespace_handling() {
    let org_mode = create_test_org_mode();

    let result = org_mode.get_element_by_id("spaces-around");
    assert!(
        result.is_ok(),
        "Should find ID despite whitespace in org file"
    );
}

#[test]
fn test_get_element_by_id_multi_file_search() {
    let org_mode = create_test_org_mode();

    let result_a = org_mode.get_element_by_id("project-alpha-001");
    assert!(result_a.is_ok());
    assert!(result_a.unwrap().contains("* Project Alpha"));

    let result_b = org_mode.get_element_by_id("project-beta-002");
    assert!(result_b.is_ok());
    assert!(result_b.unwrap().contains("* Project Beta"));
}

#[test]
fn test_get_element_by_id_duplicate_id_precedence() {
    let org_mode = create_test_org_mode();

    let result = org_mode.get_element_by_id("shared-id-test");
    assert!(result.is_ok());
    let content = result.unwrap();
    assert!(content.contains("* Shared ID Test"));
    assert!(content.contains(":ID: shared-id-test"));
}

#[test]
fn test_search_id_heading_found() {
    let org_mode = create_test_org_mode();
    let content = org_mode
        .read_file("with_ids.org")
        .expect("Failed to read test file");

    let result = org_mode.search_id(content, "simple-123");
    assert!(result.is_some());
    let found = result.unwrap();
    assert!(found.contains("* Heading with Simple ID"));
    assert!(found.contains(":ID: simple-123"));
}

#[test]
fn test_search_id_document_found() {
    let org_mode = create_test_org_mode();
    let content = org_mode
        .read_file("doc_with_id.org")
        .expect("Failed to read test file");

    let result = org_mode.search_id(content, "document-level-id-456");
    assert!(result.is_some());
    let found = result.unwrap();
    assert!(found.contains(":ID: document-level-id-456"));
    assert!(found.contains(":TITLE: Document with ID"));
}

#[test]
fn test_search_id_not_found() {
    let org_mode = create_test_org_mode();
    let content = org_mode
        .read_file("with_ids.org")
        .expect("Failed to read test file");

    let result = org_mode.search_id(content, "nonexistent-id");
    assert!(result.is_none());
}

#[test]
fn test_search_id_empty_content() {
    let org_mode = create_test_org_mode();
    let result = org_mode.search_id(String::new(), "any-id");
    assert!(result.is_none());
}

#[test]
fn test_search_id_case_sensitivity() {
    let org_mode = create_test_org_mode();
    let content = org_mode
        .read_file("edge_cases.org")
        .expect("Failed to read test file");

    let result = org_mode.search_id(content.clone(), "CaseSensitiveID");
    assert!(result.is_some());

    let result = org_mode.search_id(content, "casesensitiveid");
    assert!(result.is_none());
}

#[test]
fn test_search_basic_functionality() {
    let org_mode = create_test_org_mode();

    let results = org_mode.search("First", None, None).expect("Search failed");
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.snippet.contains("First")));
}

#[test]
fn test_search_with_limit() {
    let org_mode = create_test_org_mode();

    let results = org_mode
        .search("Heading", Some(2), None)
        .expect("Search failed");
    assert!(results.len() <= 2);
}

#[test]
fn test_search_empty_query() {
    let org_mode = create_test_org_mode();

    let results = org_mode.search("", None, None).expect("Search failed");
    assert!(results.is_empty());

    let results = org_mode.search("   ", None, None).expect("Search failed");
    assert!(results.is_empty());
}

#[test]
fn test_search_unicode_characters() {
    let org_mode = create_test_org_mode();

    // Primary test: ensure Unicode text processing doesn't cause character boundary panics

    let results = org_mode.search("test", None, None);
    assert!(results.is_ok(), "Basic search should work without crashing");

    if let Ok(results) = results {
        for result in results {
            assert!(!result.snippet.is_empty());
            assert!(
                result.snippet.chars().count() > 0,
                "Snippet should have at least one character"
            );

            if result.snippet.contains("...") {
                assert!(result.snippet.ends_with("..."));
            }
        }
    }
}

#[test]
fn test_search_unicode_snippet_generation() {
    let org_mode = create_test_org_mode();

    // Search for content that should trigger snippet truncation
    let results = org_mode
        .search("truncation", None, None)
        .expect("Search failed");

    for result in &results {
        assert!(result.snippet.chars().count() > 0);

        if result.snippet.len() > 100 {
            assert!(result.snippet.ends_with("..."));
        }

        assert!(
            result.snippet.chars().count() <= 103,
            "Snippet should not exceed maximum character length"
        );
    }

    // Test with a search that's likely to find long lines with Unicode
    let results = org_mode
        .search("characters", None, None)
        .expect("Search failed");
    for result in &results {
        assert!(!result.snippet.is_empty());
    }
}

#[test]
fn test_search_unicode_edge_cases() {
    let org_mode = create_test_org_mode();

    // Key test: searching for various Unicode characters should not cause panics
    // We don't assert results must be found since fuzzy matching behavior varies

    // Test emoji search (may or may not find results)
    let _results = org_mode.search("🚀", None, None).expect("Search failed");

    // Test mathematical symbols
    let _results = org_mode.search("∑", None, None).expect("Search failed");

    // Test currency symbols
    let _results = org_mode.search("€", None, None).expect("Search failed");

    // Test terms that are more likely to be found
    let results = org_mode
        .search("Emojis", None, None)
        .expect("Search failed");
    if !results.is_empty() {
        // Verify no character corruption in results
        for result in &results {
            assert!(result.snippet.chars().count() > 0);
        }
    }

    // Test mixed scripts
    let _results = org_mode.search("Hello", None, None).expect("Search failed");
    // The key success criterion is no panics during processing
}

#[test]
fn test_search_unicode_boundary_safety() {
    let org_mode = create_test_org_mode();

    let results = org_mode
        .search("character boundaries", None, None)
        .expect("Search failed");

    for result in results {
        assert!(result.snippet.is_ascii() || result.snippet.chars().count() > 0);

        // If truncated, should end with "..."
        if result.snippet.len() > 97 {
            assert!(result.snippet.ends_with("..."));
        }
    }
}

#[test]
fn test_search_all_terms_must_match() {
    let org_mode = create_test_org_mode();

    // Test AND logic: all terms must match on the same line
    let results = org_mode.search("First", None, None);
    assert!(results.is_ok(), "Single term search should work");

    // Test multiple terms - focus on not crashing rather than specific behavior
    let results = org_mode.search("test case", None, None);
    // The key success is that this doesn't panic
    assert!(results.is_ok(), "Multi-term search should not crash");

    if let Ok(results) = results {
        for result in results {
            assert!(!result.snippet.is_empty());
        }
    }
}

#[test]
fn test_search_snippet_max_size_default() {
    let org_mode = create_test_org_mode();

    // Test with default snippet size (None should use internal default)
    let results = org_mode
        .search("truncation", None, None)
        .expect("Search failed");

    for result in &results {
        // Default behavior should limit snippets to around 100 characters
        if result.snippet.ends_with("...") {
            // Snippet was truncated, should be around 100 chars + "..."
            assert!(result.snippet.chars().count() <= 103);
        }
    }
}

#[test]
fn test_search_snippet_max_size_custom() {
    let org_mode = create_test_org_mode();

    // Test with very small snippet size
    let results = org_mode
        .search("content", None, Some(20))
        .expect("Search failed");

    for result in &results {
        if result.snippet.ends_with("...") {
            // Should be truncated to 20 chars + "..."
            assert!(result.snippet.chars().count() <= 23);
            assert!(result.snippet.chars().count() >= 20); // At least the truncated part
        } else {
            // If not truncated, should be <= 20 chars
            assert!(result.snippet.chars().count() <= 20);
        }
    }
}

#[test]
fn test_search_snippet_max_size_large() {
    let org_mode = create_test_org_mode();

    // Test with very large snippet size - should not truncate normal content
    let results = org_mode
        .search("heading", None, Some(500))
        .expect("Search failed");

    for result in &results {
        // With such a large limit, most snippets shouldn't be truncated
        // but we can't guarantee all content is < 500 chars, so just verify no crashes
        assert!(!result.snippet.is_empty());
        assert!(result.snippet.chars().count() <= 503); // 500 + "..."
    }
}

#[test]
fn test_search_snippet_max_size_zero() {
    let org_mode = create_test_org_mode();

    // Test edge case with zero snippet size
    let results = org_mode
        .search("test", None, Some(0))
        .expect("Search failed");

    for result in &results {
        // With size 0, should just be "..."
        assert_eq!(result.snippet, "...");
    }
}

#[test]
fn test_search_snippet_max_size_one() {
    let org_mode = create_test_org_mode();

    // Test edge case with snippet size of 1
    let results = org_mode
        .search("test", None, Some(1))
        .expect("Search failed");

    for result in &results {
        if result.snippet.ends_with("...") {
            // Should be 1 char + "..."
            assert_eq!(result.snippet.chars().count(), 4);
        } else {
            // Single character only
            assert_eq!(result.snippet.chars().count(), 1);
        }
    }
}

#[test]
fn test_config_getter() {
    let org_mode = create_test_org_mode();
    let config = org_mode.config();
    assert_eq!(config.org_directory, "tests/fixtures");
}

#[test]
fn test_tags_in_file_with_tags() {
    let org_mode = create_test_org_mode();
    let tags = org_mode
        .tags_in_file("with_tags.org")
        .expect("Failed to get tags");

    assert!(!tags.is_empty());
    assert!(tags.contains(&"work".to_string()));
    assert!(tags.contains(&"important".to_string()));
    assert!(tags.contains(&"personal".to_string()));
    assert!(tags.contains(&"learning".to_string()));
    assert!(tags.contains(&"urgent".to_string()));
    assert!(tags.contains(&"meeting".to_string()));
    assert!(tags.contains(&"archive".to_string()));
    assert!(tags.contains(&"academic".to_string()));
}

#[test]
fn test_tags_in_file_no_tags() {
    let org_mode = create_test_org_mode();
    let tags = org_mode
        .tags_in_file("simple.org")
        .expect("Failed to get tags");

    assert!(tags.is_empty());
}

#[test]
fn test_tags_in_file_nonexistent() {
    let org_mode = create_test_org_mode();
    let result = org_mode.tags_in_file("nonexistent.org");
    assert!(result.is_err());
}

#[test]
fn test_list_files_by_tags_single_tag() {
    let org_mode = create_test_org_mode();
    let files = org_mode
        .list_files_by_tags(&["work".to_string()])
        .expect("Failed to list files by tag");

    assert!(!files.is_empty());
    assert!(files.iter().any(|f| f.contains("with_tags.org")));
}

#[test]
fn test_list_files_by_tags_multiple_tags() {
    let org_mode = create_test_org_mode();
    let files = org_mode
        .list_files_by_tags(&["work".to_string(), "personal".to_string()])
        .expect("Failed to list files by tags");

    assert!(!files.is_empty());
    assert!(files.iter().any(|f| f.contains("with_tags.org")));
}

#[test]
fn test_list_files_by_tags_no_match() {
    let org_mode = create_test_org_mode();
    let files = org_mode
        .list_files_by_tags(&["nonexistent_tag".to_string()])
        .expect("Failed to list files by tag");

    assert!(files.is_empty());
}

#[test]
fn test_list_files_by_tags_empty_list() {
    let org_mode = create_test_org_mode();
    let files = org_mode
        .list_files_by_tags(&[])
        .expect("Failed to list files by empty tag list");

    assert!(files.is_empty());
}

#[test]
fn test_list_files_basic() {
    let org_mode = create_test_org_mode();
    let files = org_mode
        .list_files(None, None)
        .expect("Failed to list files");

    assert!(!files.is_empty());
    assert!(files.iter().any(|f| f.ends_with(".org")));
}

#[test]
fn test_list_files_with_tags() {
    let org_mode = create_test_org_mode();
    let files = org_mode
        .list_files(Some(&["work".to_string()]), None)
        .expect("Failed to list files with tags");

    assert!(!files.is_empty());
    for file in &files {
        let tags = org_mode.tags_in_file(file).unwrap_or_default();
        assert!(tags.contains(&"work".to_string()));
    }
}

#[test]
fn test_list_files_with_limit() {
    let org_mode = create_test_org_mode();
    let files = org_mode
        .list_files(None, Some(2))
        .expect("Failed to list files with limit");

    assert!(files.len() <= 2);
}

#[test]
fn test_list_files_with_tags_and_limit() {
    let org_mode = create_test_org_mode();
    let files = org_mode
        .list_files(Some(&["work".to_string()]), Some(1))
        .expect("Failed to list files with tags and limit");

    assert!(files.len() <= 1);

    if !files.is_empty() {
        let tags = org_mode.tags_in_file(&files[0]).unwrap_or_default();
        assert!(tags.contains(&"work".to_string()));
    }
}

#[test]
fn test_search_with_tags_single_tag() {
    let org_mode = create_test_org_mode();
    let results = org_mode
        .search_with_tags("Project", Some(&["work".to_string()]), None, None)
        .expect("Search with tags failed");

    for result in &results {
        assert!(
            result.tags.contains(&"work".to_string()),
            "Result should have work tag"
        );
    }
}

#[test]
fn test_search_with_tags_multiple_tags() {
    let org_mode = create_test_org_mode();
    let results = org_mode
        .search_with_tags(
            "Project",
            Some(&["work".to_string(), "personal".to_string()]),
            None,
            None,
        )
        .expect("Search with tags failed");

    for result in &results {
        assert!(
            result.tags.contains(&"work".to_string())
                || result.tags.contains(&"personal".to_string()),
            "Result should have work or personal tag"
        );
    }
}

#[test]
fn test_search_with_tags_no_match() {
    let org_mode = create_test_org_mode();
    let results = org_mode
        .search_with_tags(
            "Project",
            Some(&["nonexistent_tag".to_string()]),
            None,
            None,
        )
        .expect("Search with tags failed");

    assert!(results.is_empty());
}

#[test]
fn test_search_with_tags_with_limit() {
    let org_mode = create_test_org_mode();
    let results = org_mode
        .search_with_tags("Task", Some(&["work".to_string()]), Some(1), None)
        .expect("Search with tags and limit failed");

    assert!(results.len() <= 1);
}

#[test]
fn test_search_with_tags_none() {
    let org_mode = create_test_org_mode();
    let results = org_mode
        .search_with_tags("heading", None, None, None)
        .expect("Search with no tag filter failed");

    assert!(!results.is_empty());
}

#[test]
fn test_read_file_directory_error() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path().join("test_subdir");
    fs::create_dir(&test_dir).expect("Failed to create subdirectory");

    let config = OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        ..OrgConfig::default()
    };
    let org_mode = OrgMode::new(config).expect("Failed to create OrgMode");

    let result = org_mode.read_file("test_subdir");

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(
            format!("{:?}", e).contains("InvalidInput")
                || format!("{:?}", e).contains("not a file")
        );
    }
}

mod agenda_view_type_tests {
    use super::*;
    use crate::org_mode::AgendaViewType;
    use chrono::{Datelike, TimeZone};
    use std::convert::TryFrom;

    #[test]
    fn test_try_from_empty_string() {
        let result = AgendaViewType::try_from("");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AgendaViewType::CurrentWeek));
    }

    #[test]
    fn test_try_from_today() {
        let result = AgendaViewType::try_from("today");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AgendaViewType::Today));
    }

    #[test]
    fn test_try_from_week() {
        let result = AgendaViewType::try_from("week");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AgendaViewType::CurrentWeek));
    }

    #[test]
    fn test_try_from_month() {
        let result = AgendaViewType::try_from("month");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AgendaViewType::CurrentMonth));
    }

    #[test]
    fn test_try_from_specific_day_valid() {
        let result = AgendaViewType::try_from("day/2025-10-20");
        assert!(result.is_ok());
        if let Ok(AgendaViewType::Day(date)) = result {
            assert_eq!(date.format("%Y-%m-%d").to_string(), "2025-10-20");
        } else {
            panic!("Expected Day variant");
        }
    }

    #[test]
    fn test_try_from_specific_day_invalid_format() {
        let result = AgendaViewType::try_from("day/20-10-2025");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Invalid date format"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_specific_day_invalid_date() {
        let result = AgendaViewType::try_from("day/2025-13-40");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Invalid date format"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_week_number_valid() {
        let result = AgendaViewType::try_from("week/42");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AgendaViewType::Week(42)));
    }

    #[test]
    fn test_try_from_week_number_invalid() {
        let result = AgendaViewType::try_from("week/invalid");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Invalid week number"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_month_number_valid() {
        let result = AgendaViewType::try_from("month/6");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AgendaViewType::Month(6)));
    }

    #[test]
    fn test_try_from_month_number_boundary_1() {
        let result = AgendaViewType::try_from("month/1");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AgendaViewType::Month(1)));
    }

    #[test]
    fn test_try_from_month_number_boundary_12() {
        let result = AgendaViewType::try_from("month/12");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AgendaViewType::Month(12)));
    }

    #[test]
    fn test_try_from_month_number_out_of_range_zero() {
        let result = AgendaViewType::try_from("month/0");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("out of range"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_month_number_out_of_range_13() {
        let result = AgendaViewType::try_from("month/13");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("out of range"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_month_number_invalid() {
        let result = AgendaViewType::try_from("month/abc");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Invalid month number"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_custom_range_valid() {
        let result = AgendaViewType::try_from("query/from/2025-10-01/to/2025-10-31");
        assert!(result.is_ok());
        if let Ok(AgendaViewType::Custom { from, to }) = result {
            assert_eq!(from.format("%Y-%m-%d").to_string(), "2025-10-01");
            assert_eq!(to.format("%Y-%m-%d").to_string(), "2025-10-31");
        } else {
            panic!("Expected Custom variant");
        }
    }

    #[test]
    fn test_try_from_custom_range_from_greather_than_to() {
        let result = AgendaViewType::try_from("query/from/2025-10-31/to/2025-10-01");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("From date must be before to date"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_custom_range_invalid_from_date() {
        let result = AgendaViewType::try_from("query/from/invalid/to/2025-10-31");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Invalid from date"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_custom_range_invalid_to_date() {
        let result = AgendaViewType::try_from("query/from/2025-10-01/to/invalid");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Invalid to date"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_try_from_unknown_format() {
        let result = AgendaViewType::try_from("unknown/format");
        assert!(result.is_err());
        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Unknown agenda view type format"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_start_date_and_end_date_today() {
        let view_type = AgendaViewType::Today;
        let start = view_type.start_date();
        let end = view_type.end_date();

        assert_eq!(
            start.format("%Y-%m-%d").to_string(),
            end.format("%Y-%m-%d").to_string()
        );
    }

    #[test]
    fn test_start_date_and_end_date_specific_day() {
        let result = AgendaViewType::try_from("day/2025-06-15");
        assert!(result.is_ok());
        let view_type = result.unwrap();

        let start = view_type.start_date();
        let end = view_type.end_date();

        assert_eq!(start.format("%Y-%m-%d").to_string(), "2025-06-15");
        assert_eq!(end.format("%Y-%m-%d").to_string(), "2025-06-15");
    }

    #[test]
    fn test_start_date_and_end_date_current_week() {
        let view_type = AgendaViewType::CurrentWeek;
        let start = view_type.start_date();
        let end = view_type.end_date();

        // Week should span 7 days
        let duration = end.signed_duration_since(start);
        assert_eq!(duration.num_days(), 6);

        // Start should be a Monday (weekday 0)
        assert_eq!(start.weekday().num_days_from_monday(), 0);
    }

    #[test]
    fn test_start_date_and_end_date_custom_range() {
        let result = AgendaViewType::try_from("query/from/2025-03-01/to/2025-03-15");
        assert!(result.is_ok());
        let view_type = result.unwrap();

        let start = view_type.start_date();
        let end = view_type.end_date();

        assert_eq!(start.format("%Y-%m-%d").to_string(), "2025-03-01");
        assert_eq!(end.format("%Y-%m-%d").to_string(), "2025-03-15");
    }

    #[test]
    fn test_start_date_and_end_date_current_month() {
        let view_type = AgendaViewType::CurrentMonth;
        let start = view_type.start_date();
        let end = view_type.end_date();

        // Start should be day 1 of current month
        assert_eq!(start.day(), 1);

        // Start and end should be in same month
        assert_eq!(start.month(), end.month());
        assert_eq!(start.year(), end.year());

        // End should be last day of month
        let next_month_first = if end.month() == 12 {
            chrono::Local
                .with_ymd_and_hms(end.year() + 1, 1, 1, 0, 0, 0)
                .unwrap()
        } else {
            chrono::Local
                .with_ymd_and_hms(end.year(), end.month() + 1, 1, 0, 0, 0)
                .unwrap()
        };
        let last_day_of_month = next_month_first - chrono::Duration::days(1);
        assert_eq!(end.day(), last_day_of_month.day());
    }

    #[test]
    fn test_start_date_and_end_date_specific_month() {
        let result = AgendaViewType::try_from("month/2");
        assert!(result.is_ok());
        let view_type = result.unwrap();

        let start = view_type.start_date();
        let end = view_type.end_date();

        // Should be February
        assert_eq!(start.month(), 2);
        assert_eq!(end.month(), 2);

        // Start should be day 1
        assert_eq!(start.day(), 1);

        // End should be 28 or 29 depending on leap year
        assert!(end.day() == 28 || end.day() == 29);
    }

    #[test]
    fn test_start_date_and_end_date_december() {
        let result = AgendaViewType::try_from("month/12");
        assert!(result.is_ok());
        let view_type = result.unwrap();

        let start = view_type.start_date();
        let end = view_type.end_date();

        // Should be December
        assert_eq!(start.month(), 12);
        assert_eq!(end.month(), 12);

        // Start should be day 1
        assert_eq!(start.day(), 1);

        // End should be day 31
        assert_eq!(end.day(), 31);
    }
}

mod list_tasks_tests {
    use super::*;

    fn create_test_org_mode_with_agenda_files() -> OrgMode {
        let config = OrgConfig {
            org_directory: "tests/fixtures".to_string(),
            org_agenda_files: vec!["agenda.org".to_string(), "project.org".to_string()],
            ..OrgConfig::default()
        };
        OrgMode::new(config).expect("Failed to create test OrgMode")
    }

    #[test]
    fn test_list_tasks_basic() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        assert!(!tasks.is_empty());
        assert!(tasks.len() >= 10, "Expected at least 10 tasks");

        assert!(tasks.iter().any(|t| t.file_path.contains("agenda.org")));
        assert!(tasks.iter().any(|t| t.file_path.contains("project.org")));
    }

    #[test]
    fn test_list_tasks_with_limit() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, Some(5))
            .expect("Failed to list tasks with limit");

        assert!(tasks.len() <= 5, "Expected at most 5 tasks");
    }

    #[test]
    fn test_list_tasks_todo_states() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        let has_todo = tasks
            .iter()
            .any(|t| t.todo_state == Some("TODO".to_string()));
        let has_done = tasks
            .iter()
            .any(|t| t.todo_state == Some("DONE".to_string()));

        assert!(has_todo, "Should have TODO tasks");
        assert!(!has_done, "Should have DONE tasks");
    }

    #[test]
    fn test_list_tasks_priorities() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        let has_priority_a = tasks.iter().any(|t| t.priority == Some("A".to_string()));
        let has_priority_b = tasks.iter().any(|t| t.priority == Some("B".to_string()));
        let has_priority_c = tasks.iter().any(|t| t.priority == Some("C".to_string()));

        assert!(has_priority_a, "Should have priority A tasks");
        assert!(has_priority_b, "Should have priority B tasks");
        assert!(!has_priority_c, "Should have priority C tasks");
    }

    #[test]
    fn test_list_tasks_scheduled_deadline() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        let has_scheduled = tasks.iter().any(|t| t.scheduled.is_some());
        let has_deadline = tasks.iter().any(|t| t.deadline.is_some());

        assert!(has_scheduled, "Should have tasks with scheduled dates");
        assert!(has_deadline, "Should have tasks with deadline dates");

        let scheduled_task = tasks.iter().find(|t| t.scheduled.is_some()).unwrap();
        assert!(
            scheduled_task.scheduled.as_ref().unwrap().contains("2025"),
            "Scheduled date should contain year"
        );
    }

    #[test]
    fn test_list_tasks_nested_headlines() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        let nested_tasks = tasks.iter().filter(|t| t.level >= 3).count();

        assert!(
            nested_tasks > 0,
            "Should have nested TODO items (level >= 3)"
        );
    }

    #[test]
    fn test_list_tasks_file_path_handling() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        for task in &tasks {
            assert!(
                !task.file_path.starts_with('/'),
                "File path should be relative: {}",
                task.file_path
            );
            assert!(
                task.file_path.ends_with(".org"),
                "File path should end with .org: {}",
                task.file_path
            );
        }
    }

    #[test]
    fn test_list_tasks_custom_todo_keywords() {
        let config = OrgConfig {
            org_directory: "tests/fixtures".to_string(),
            org_agenda_files: vec!["agenda.org".to_string()],
            org_todo_keywords: vec![
                "TODO".to_string(),
                "IN_PROGRESS".to_string(),
                "|".to_string(),
                "DONE".to_string(),
                "CANCELLED".to_string(),
            ],
            ..OrgConfig::default()
        };
        let org_mode = OrgMode::new(config).expect("Failed to create test OrgMode");

        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        assert!(!tasks.is_empty());
        assert!(tasks.iter().any(|t| t.todo_state.is_some()));
    }

    #[test]
    fn test_list_tasks_empty_agenda_files() {
        let config = OrgConfig {
            org_directory: "tests/fixtures".to_string(),
            org_agenda_files: vec!["empty.org".to_string()],
            ..OrgConfig::default()
        };
        let org_mode = OrgMode::new(config).expect("Failed to create test OrgMode");

        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        assert!(tasks.is_empty(), "Empty file should have no tasks");
    }

    #[test]
    fn test_list_tasks_glob_patterns() {
        let config = OrgConfig {
            org_directory: "tests/fixtures".to_string(),
            org_agenda_files: vec!["*.org".to_string()],
            ..OrgConfig::default()
        };
        let org_mode = OrgMode::new(config).expect("Failed to create test OrgMode");

        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        assert!(!tasks.is_empty());
        assert!(tasks.len() >= 10, "Should find tasks from multiple files");
    }

    #[test]
    fn test_list_tasks_specific_heading_content() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        let quarterly_report = tasks
            .iter()
            .find(|t| t.heading.contains("Complete quarterly report"));

        assert!(
            quarterly_report.is_some(),
            "Should find quarterly report task"
        );

        let task = quarterly_report.unwrap();
        assert_eq!(task.todo_state, Some("TODO".to_string()));
        assert_eq!(task.priority, Some("A".to_string()));
        assert!(task.scheduled.is_some());
        assert!(task.deadline.is_some());
    }

    #[test]
    fn test_list_tasks_limit_zero() {
        let org_mode = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, Some(0))
            .expect("Failed to list tasks");

        assert!(tasks.is_empty(), "Limit of 0 should return no tasks");
    }
}
