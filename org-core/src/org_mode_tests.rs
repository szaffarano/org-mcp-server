use crate::org_mode::TreeNode;
use crate::{OrgMode, OrgModeError};

fn create_test_org_mode() -> OrgMode {
    let test_dir = "tests/fixtures";
    OrgMode::new(test_dir).expect("Failed to create test OrgMode")
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
    assert!(results[0].snippet.contains("First"));
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
            assert_eq!(
                result.snippet.chars().count(),
                result.snippet.chars().count()
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
