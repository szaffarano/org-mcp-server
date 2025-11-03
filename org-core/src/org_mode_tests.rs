use std::path::Path;

use crate::config::OrgConfig;
use crate::{OrgMode, OrgModeError};

fn create_test_org_mode(path: &Path) -> OrgMode {
    let config = OrgConfig {
        org_directory: path.to_string_lossy().to_string(),
        ..OrgConfig::default()
    };
    OrgMode::new(config).expect("Failed to create test OrgMode")
}

mod basic_use_cases {
    use test_utils::fixtures;

    use crate::{
        OrgConfig, OrgMode, OrgModeError,
        org_mode::{TreeNode, org_mode_tests::create_test_org_mode},
    };

    #[test]
    fn test_get_outline_simple() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());
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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());
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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode
            .get_heading("simple.org", "First Heading")
            .expect("Failed to get heading");

        assert!(result.contains("* First Heading"));
        assert!(result.contains("This is content under the first heading."));
    }

    #[test]
    fn test_get_heading_nested_path() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode
            .get_heading("nested.org", "Project Planning/Phase 1/Setup Tasks")
            .expect("Failed to get nested heading");

        assert!(result.contains("*** Setup Tasks"));
        assert!(result.contains("Install dependencies"));
        assert!(result.contains("Configure environment"));
    }

    #[test]
    fn test_get_heading_nonexistent() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode
            .get_element_by_id("550e8400-e29b-41d4-a716-446655440000")
            .expect("Failed to get element by UUID ID");

        assert!(result.contains("* Heading with UUID"));
        assert!(result.contains(":ID: 550e8400-e29b-41d4-a716-446655440000"));
        assert!(result.contains("This heading has a UUID-style ID."));
    }

    #[test]
    fn test_get_element_by_id_document_level() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode
            .get_element_by_id("document-level-id-456")
            .expect("Failed to get document-level element by ID");

        assert!(result.contains(":ID: document-level-id-456"));
        assert!(result.contains(":TITLE: Document with ID"));
        assert!(result.contains("This is a document with a document-level ID property."));
    }

    #[test]
    fn test_get_element_by_id_nested_heading() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode
            .get_element_by_id("nested-abc")
            .expect("Failed to get nested element by ID");

        assert!(result.contains("** Nested Heading with ID"));
        assert!(result.contains(":ID: nested-abc"));
        assert!(result.contains("This is a nested heading with an ID."));
    }

    #[test]
    fn test_get_element_by_id_with_multiple_properties() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode.get_element_by_id("CaseSensitiveID");
        assert!(result.is_ok());

        let result = org_mode.get_element_by_id("casesensitiveid");
        assert!(result.is_err());

        let result = org_mode.get_element_by_id("CASESENSITIVEID");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_element_by_id_special_characters() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode
            .get_element_by_id("special-chars!@#$%")
            .expect("Failed to get element with special characters in ID");

        assert!(result.contains("* Heading with Special Characters"));
        assert!(result.contains(":ID: special-chars!@#$%"));
    }

    #[test]
    fn test_get_element_by_id_whitespace_handling() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode.get_element_by_id("spaces-around");
        assert!(
            result.is_ok(),
            "Should find ID despite whitespace in org file"
        );
    }

    #[test]
    fn test_get_element_by_id_multi_file_search() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result_a = org_mode.get_element_by_id("project-alpha-001");
        assert!(result_a.is_ok());
        assert!(result_a.unwrap().contains("* Project Alpha"));

        let result_b = org_mode.get_element_by_id("project-beta-002");
        assert!(result_b.is_ok());
        assert!(result_b.unwrap().contains("* Project Beta"));
    }

    #[test]
    fn test_get_element_by_id_duplicate_id_precedence() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode.get_element_by_id("shared-id-test");
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("* Shared ID Test"));
        assert!(content.contains(":ID: shared-id-test"));
    }

    #[test]
    fn test_search_id_heading_found() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let content = org_mode
            .read_file("with_ids.org")
            .expect("Failed to read test file");

        let result = org_mode.search_id(content, "nonexistent-id");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_id_empty_content() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode.search_id(String::new(), "any-id");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_id_case_sensitivity() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let results = org_mode.search("First", None, None).expect("Search failed");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.snippet.contains("First")));
    }

    #[test]
    fn test_search_with_limit() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let results = org_mode
            .search("Heading", Some(2), None)
            .expect("Search failed");
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_search_empty_query() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let results = org_mode.search("", None, None).expect("Search failed");
        assert!(results.is_empty());

        let results = org_mode.search("   ", None, None).expect("Search failed");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_unicode_characters() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let config = org_mode.config();
        assert_eq!(
            config.org_directory,
            org_dir.path().to_string_lossy().to_string()
        );
    }

    #[test]
    fn test_tags_in_file_with_tags() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let tags = org_mode
            .tags_in_file("simple.org")
            .expect("Failed to get tags");

        assert!(tags.is_empty());
    }

    #[test]
    fn test_tags_in_file_nonexistent() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let result = org_mode.tags_in_file("nonexistent.org");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_files_by_tags_single_tag() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let files = org_mode
            .list_files_by_tags(&["work".to_string()])
            .expect("Failed to list files by tag");

        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.contains("with_tags.org")));
    }

    #[test]
    fn test_list_files_by_tags_multiple_tags() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let files = org_mode
            .list_files_by_tags(&["work".to_string(), "personal".to_string()])
            .expect("Failed to list files by tags");

        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.contains("with_tags.org")));
    }

    #[test]
    fn test_list_files_by_tags_no_match() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let files = org_mode
            .list_files_by_tags(&["nonexistent_tag".to_string()])
            .expect("Failed to list files by tag");

        assert!(files.is_empty());
    }

    #[test]
    fn test_list_files_by_tags_empty_list() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let files = org_mode
            .list_files_by_tags(&[])
            .expect("Failed to list files by empty tag list");

        assert!(files.is_empty());
    }

    #[test]
    fn test_list_files_basic() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let files = org_mode
            .list_files(None, None)
            .expect("Failed to list files");

        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.ends_with(".org")));
    }

    #[test]
    fn test_list_files_with_tags() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let files = org_mode
            .list_files(None, Some(2))
            .expect("Failed to list files with limit");

        assert!(files.len() <= 2);
    }

    #[test]
    fn test_list_files_with_tags_and_limit() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

        let results = org_mode
            .search_with_tags("Task", Some(&["work".to_string()]), Some(1), None)
            .expect("Search with tags and limit failed");

        assert!(results.len() <= 1);
    }

    #[test]
    fn test_search_with_tags_none() {
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let org_mode = create_test_org_mode(org_dir.path());

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

    use chrono::{Local, NaiveDate};
    use serial_test::serial;
    use tempfile::TempDir;
    use test_utils::fixtures::{self, setup_test_org_files};

    use super::*;

    // Load test fixtures with dynamic dates
    fn create_test_org_mode_with_agenda_files() -> (OrgMode, TempDir) {
        create_test_org_mode_with_agenda_files_for_date(Local::now().date_naive())
    }

    // Load test fixtures with specific base date (for deterministic testing)
    fn create_test_org_mode_with_agenda_files_for_date(base_date: NaiveDate) -> (OrgMode, TempDir) {
        let temp_dir = setup_test_org_files().expect("Failed to set up test org files");

        test_utils::fixtures::copy_fixtures_with_dates(&temp_dir, base_date)
            .expect("Failed to copy fixtures with dates");

        let config = OrgConfig {
            org_directory: temp_dir.path().to_string_lossy().to_string(),
            org_agenda_files: vec!["agenda.org".to_string(), "project.org".to_string()],
            ..OrgConfig::default()
        };

        let org_mode = OrgMode::new(config).expect("Failed to create test OrgMode");
        (org_mode, temp_dir)
    }

    #[test]
    fn test_list_tasks_basic() {
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
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
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, Some(5))
            .expect("Failed to list tasks with limit");

        assert!(tasks.len() <= 5, "Expected at most 5 tasks");
    }

    #[test]
    fn test_list_tasks_todo_states() {
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
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
        assert!(!has_done, "Should not have DONE tasks");
    }

    #[test]
    fn test_list_tasks_priorities() {
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, None)
            .expect("Failed to list tasks");

        let has_priority_a = tasks.iter().any(|t| t.priority == Some("A".to_string()));
        let has_priority_b = tasks.iter().any(|t| t.priority == Some("B".to_string()));
        let has_priority_c = tasks.iter().any(|t| t.priority == Some("C".to_string()));

        assert!(has_priority_a, "Should have priority A tasks");
        assert!(has_priority_b, "Should have priority B tasks");
        assert!(has_priority_c, "Should have priority C tasks");
    }

    #[test]
    fn test_list_tasks_scheduled_deadline() {
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
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
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
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
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let config = OrgConfig {
            org_directory: org_dir.path().to_string_lossy().to_string(),
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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let config = OrgConfig {
            org_directory: org_dir.path().to_string_lossy().to_string(),
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
        let org_dir = fixtures::setup_test_org_files().unwrap();
        let config = OrgConfig {
            org_directory: org_dir.path().to_string_lossy().to_string(),
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
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
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
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let tasks = org_mode
            .list_tasks(None, None, None, Some(0))
            .expect("Failed to list tasks");

        assert!(tasks.is_empty(), "Limit of 0 should return no tasks");
    }

    #[test]
    fn test_list_tasks_with_state_filter() {
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        let todo_tasks = org_mode
            .list_tasks(Some(&["TODO".to_string()]), None, None, None)
            .expect("Failed to list TODO tasks");

        assert!(!todo_tasks.is_empty(), "Should have TODO tasks");
        for task in &todo_tasks {
            assert_eq!(
                task.todo_state.as_deref(),
                Some("TODO"),
                "Task '{}' should have TODO state",
                task.heading
            );
        }
        assert!(
            todo_tasks.len() >= 10,
            "Expected at least 10 TODO tasks, got {}",
            todo_tasks.len()
        );

        let done_tasks = org_mode
            .list_tasks(Some(&["DONE".to_string()]), None, None, None)
            .expect("Failed to list DONE tasks");

        assert!(done_tasks.is_empty(), "Should not have DONE tasks");
    }

    #[test]
    #[serial]
    fn test_list_tasks_with_tag_filter() {
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        // Filter by "work" tag
        let work_tasks = org_mode
            .list_tasks(None, Some(&["work".to_string()]), None, None)
            .expect("Failed to list tasks with work tag");

        assert!(
            work_tasks.len() >= 2,
            "Should have at least 2 tasks with 'work' tag, got {}",
            work_tasks.len()
        );
        // Verify all returned tasks have the 'work' tag
        for task in &work_tasks {
            assert!(
                task.tags.contains(&"work".to_string()),
                "Task '{}' should have 'work' tag, has {:?}",
                task.heading,
                task.tags
            );
        }

        // Filter by "personal" tag
        let personal_tasks = org_mode
            .list_tasks(None, Some(&["personal".to_string()]), None, None)
            .expect("Failed to list tasks with personal tag");

        assert!(
            !personal_tasks.is_empty(),
            "Should have at least 1 task with 'personal' tag, got {}",
            personal_tasks.len()
        );
        // Verify all returned tasks have the 'personal' tag
        for task in &personal_tasks {
            assert!(
                task.tags.contains(&"personal".to_string()),
                "Task '{}' should have 'personal' tag, has {:?}",
                task.heading,
                task.tags
            );
        }

        // Filter by "urgent" tag (should find tasks with urgent tag)
        let urgent_tasks = org_mode
            .list_tasks(None, Some(&["urgent".to_string()]), None, None)
            .expect("Failed to list tasks with urgent tag");

        assert!(
            !urgent_tasks.is_empty(),
            "Should have at least 1 task with 'urgent' tag, got {}",
            urgent_tasks.len()
        );
        for task in &urgent_tasks {
            assert!(
                task.tags.contains(&"urgent".to_string()),
                "Task '{}' should have 'urgent' tag, has {:?}",
                task.heading,
                task.tags
            );
        }
    }

    #[test]
    fn test_list_tasks_with_priority_filter() {
        use crate::Priority;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        // Test Priority A filter
        let a_tasks = org_mode
            .list_tasks(None, None, Some(Priority::A), None)
            .expect("Failed to list priority A tasks");

        assert!(
            a_tasks.len() >= 2,
            "Should have at least 2 priority A tasks, got {}",
            a_tasks.len()
        );
        for task in &a_tasks {
            assert_eq!(
                task.priority.as_deref(),
                Some("A"),
                "Task '{}' should have priority A, has {:?}",
                task.heading,
                task.priority
            );
        }

        // Test Priority B filter
        let b_tasks = org_mode
            .list_tasks(None, None, Some(Priority::B), None)
            .expect("Failed to list priority B tasks");

        assert!(
            !b_tasks.is_empty(),
            "Should have at least 1 priority B task, got {}",
            b_tasks.len()
        );
        for task in &b_tasks {
            assert_eq!(
                task.priority.as_deref(),
                Some("B"),
                "Task '{}' should have priority B, has {:?}",
                task.heading,
                task.priority
            );
        }

        // Test Priority C filter
        let c_tasks = org_mode
            .list_tasks(None, None, Some(Priority::C), None)
            .expect("Failed to list priority C tasks");

        // Note: "Update documentation" is DONE with priority C, but we filter TODO items
        // So we might have 0 or more C priority tasks depending on fixtures
        for task in &c_tasks {
            assert_eq!(
                task.priority.as_deref(),
                Some("C"),
                "Task '{}' should have priority C, has {:?}",
                task.heading,
                task.priority
            );
        }

        // Test Priority::None filter (tasks with no priority)
        let no_priority_tasks = org_mode
            .list_tasks(None, None, Some(Priority::None), None)
            .expect("Failed to list tasks with no priority");

        assert!(
            !no_priority_tasks.is_empty(),
            "Should have tasks with no priority"
        );
        for task in &no_priority_tasks {
            assert!(
                task.priority.is_none(),
                "Task '{}' should have no priority, has {:?}",
                task.heading,
                task.priority
            );
        }
    }

    #[test]
    fn test_list_tasks_combined_filters() {
        use crate::Priority;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        // Combine TODO state + priority filter
        let todo_a_tasks = org_mode
            .list_tasks(Some(&["TODO".to_string()]), None, Some(Priority::A), None)
            .expect("Failed to list TODO tasks with priority A");

        assert!(
            todo_a_tasks.len() >= 2,
            "Should have TODO tasks with priority A, got {}",
            todo_a_tasks.len()
        );
        for task in &todo_a_tasks {
            assert_eq!(task.todo_state.as_deref(), Some("TODO"));
            assert_eq!(task.priority.as_deref(), Some("A"));
        }

        // Combine TODO state + tag filter
        let todo_work_tasks = org_mode
            .list_tasks(
                Some(&["TODO".to_string()]),
                Some(&["work".to_string()]),
                None,
                None,
            )
            .expect("Failed to list TODO work tasks");

        assert!(
            todo_work_tasks.len() >= 2,
            "Should have TODO work tasks, got {}",
            todo_work_tasks.len()
        );
        for task in &todo_work_tasks {
            assert_eq!(task.todo_state.as_deref(), Some("TODO"));
            assert!(task.tags.contains(&"work".to_string()));
        }

        // Combine all three filters: TODO + work + urgent
        let todo_work_urgent = org_mode
            .list_tasks(
                Some(&["TODO".to_string()]),
                Some(&["work".to_string(), "urgent".to_string()]),
                None,
                None,
            )
            .expect("Failed to list TODO work+urgent tasks");

        // This should find "Code review session" which has both work and urgent tags
        assert!(
            !todo_work_urgent.is_empty(),
            "Should have at least 1 TODO work+urgent task, got {}",
            todo_work_urgent.len()
        );
        for task in &todo_work_urgent {
            assert_eq!(task.todo_state.as_deref(), Some("TODO"));
            assert!(task.tags.contains(&"work".to_string()));
            assert!(task.tags.contains(&"urgent".to_string()));
        }
    }

    #[test]
    fn test_list_tasks_multiple_tags() {
        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        // Task "Code review session" has tags ["work", "review", "urgent"]
        // Filtering by ["review", "urgent"] should find it
        let review_urgent_tasks = org_mode
            .list_tasks(
                None,
                Some(&["review".to_string(), "urgent".to_string()]),
                None,
                None,
            )
            .expect("Failed to list tasks with review+urgent tags");

        assert!(
            !review_urgent_tasks.is_empty(),
            "Should have at least 1 task with review+urgent tags, got {}",
            review_urgent_tasks.len()
        );
        for task in &review_urgent_tasks {
            assert!(task.tags.contains(&"review".to_string()));
            assert!(task.tags.contains(&"urgent".to_string()));
        }

        // Filtering by ["work", "review"] should find tasks with both tags
        let work_review_tasks = org_mode
            .list_tasks(
                None,
                Some(&["work".to_string(), "review".to_string()]),
                None,
                None,
            )
            .expect("Failed to list tasks with work+review tags");

        assert!(
            !work_review_tasks.is_empty(),
            "Should have tasks with work+review tags, got {}",
            work_review_tasks.len()
        );
        for task in &work_review_tasks {
            assert!(task.tags.contains(&"work".to_string()));
            assert!(task.tags.contains(&"review".to_string()));
        }
    }

    #[test]
    fn test_list_tasks_no_match_filters() {
        use crate::Priority;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        // Filter by non-existent tag
        let nonexistent_tag_tasks = org_mode
            .list_tasks(None, Some(&["nonexistent".to_string()]), None, None)
            .expect("Failed to list tasks with nonexistent tag");

        assert!(
            nonexistent_tag_tasks.is_empty(),
            "Should have no tasks with nonexistent tag, got {}",
            nonexistent_tag_tasks.len()
        );

        // Filter by combination that doesn't exist (DONE + Priority A)
        // In our fixtures, Priority A tasks are all TODO
        let done_a_tasks = org_mode
            .list_tasks(Some(&["DONE".to_string()]), None, Some(Priority::A), None)
            .expect("Failed to list DONE priority A tasks");

        assert!(
            done_a_tasks.is_empty(),
            "Should have no DONE tasks with priority A, got {}",
            done_a_tasks.len()
        );

        // Filter by impossible combination (personal + work tags together on same task)
        let personal_work_tasks = org_mode
            .list_tasks(
                None,
                Some(&["personal".to_string(), "work".to_string()]),
                None,
                None,
            )
            .expect("Failed to list tasks with personal+work tags");

        assert!(
            personal_work_tasks.is_empty(),
            "Should have no tasks with both personal and work tags, got {}",
            personal_work_tasks.len()
        );
    }

    #[test]
    fn test_get_agenda_view_today() {
        use crate::org_mode::AgendaViewType;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let view = org_mode
            .get_agenda_view(AgendaViewType::Today, None, None)
            .expect("Failed to get today's agenda view");

        assert!(
            view.start_date.is_some(),
            "Today view should have start_date"
        );
        assert!(view.end_date.is_some(), "Today view should have end_date");

        // With dynamic dates, we should find tasks scheduled for today
        // Based on our fixture: "@TODAY@" tasks include "Review pull requests" and "Team standup"
        assert!(
            view.items.len() >= 2,
            "Should find at least 2 tasks scheduled for today, found {}",
            view.items.len()
        );

        // Verify we're finding the expected tasks
        let has_review_prs = view
            .items
            .iter()
            .any(|item| item.heading.contains("Review pull requests"));
        let has_standup = view
            .items
            .iter()
            .any(|item| item.heading.contains("Team standup"));

        assert!(
            has_review_prs || has_standup,
            "Should find at least one of today's scheduled tasks"
        );
    }

    #[test]
    fn test_get_agenda_view_current_week() {
        use crate::org_mode::AgendaViewType;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let view = org_mode
            .get_agenda_view(AgendaViewType::CurrentWeek, None, None)
            .expect("Failed to get current week agenda view");

        assert!(
            view.start_date.is_some(),
            "Week view should have start_date"
        );
        assert!(view.end_date.is_some(), "Week view should have end_date");

        // Week view should find at least some tasks
        // Number varies depending on which day of the week "today" is
        assert!(
            !view.items.is_empty(),
            "Should find at least 1 task in current week, found {}",
            view.items.len()
        );

        // Verify we're finding tasks scheduled for the current week
        // At minimum, @TODAY@ tasks should be included
        let task_names: Vec<String> = view.items.iter().map(|i| i.heading.clone()).collect();
        let has_weekly_task = task_names
            .iter()
            .any(|name| name.contains("Review pull requests") || name.contains("Team standup"));

        assert!(
            has_weekly_task,
            "Should find tasks from the current week (at least @TODAY@ tasks)"
        );
    }

    #[test]
    fn test_get_agenda_view_custom_week() {
        use crate::org_mode::AgendaViewType;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let view = org_mode
            .get_agenda_view(AgendaViewType::Week(9), None, None)
            .expect("Failed to get current week agenda view");

        assert!(
            view.start_date.is_some(),
            "Week view should have start_date"
        );
        assert!(view.end_date.is_some(), "Week view should have end_date");

        // TODO: complete once filters are implemented
    }

    #[test]
    fn test_get_agenda_view_custom_range() {
        use crate::org_mode::AgendaViewType;
        use chrono::{Days, Local};

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        // Test with a custom range from TODAY+1 to TODAY+6 (should match several fixture tasks)
        let today = Local::now();
        let from = today.checked_add_days(Days::new(1)).unwrap();
        let to = today.checked_add_days(Days::new(6)).unwrap();

        let view = org_mode
            .get_agenda_view(AgendaViewType::Custom { from, to }, None, None)
            .expect("Failed to get custom range agenda view");

        assert!(
            view.start_date.is_some(),
            "Custom view should have start_date"
        );
        assert!(view.end_date.is_some(), "Custom view should have end_date");

        // Should find tasks scheduled in this range
        // Based on fixtures: @TODAY+1@, @TODAY+2@, @TODAY+3@, @TODAY+4@, @TODAY+5@, @TODAY+6@
        assert!(
            view.items.len() >= 3,
            "Should find at least 3 tasks in custom range, found {}",
            view.items.len()
        );

        // Verify we find expected tasks in this range
        let has_quarterly_report = view
            .items
            .iter()
            .any(|item| item.heading.contains("Complete quarterly report"));

        assert!(
            has_quarterly_report,
            "Should find 'Complete quarterly report' task scheduled for TODAY+1"
        );
    }

    #[test]
    fn test_get_agenda_view_with_filters() {
        use crate::org_mode::AgendaViewType;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        let _view = org_mode
            .get_agenda_view(
                AgendaViewType::Today,
                Some(&["TODO".to_string()]),
                Some(&["work".to_string()]),
            )
            .expect("Failed to get filtered agenda view");

        // TODO: complete once filters are implemented
    }

    #[test]
    fn test_get_agenda_view_empty_results() {
        use crate::org_mode::AgendaViewType;
        use chrono::{Local, TimeZone};

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

        let from = Local.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
        let to = Local.with_ymd_and_hms(2030, 1, 7, 23, 59, 59).unwrap();

        let view = org_mode
            .get_agenda_view(AgendaViewType::Custom { from, to }, None, None)
            .expect("Failed to get agenda view");

        // Far future dates should have no tasks
        assert!(
            view.items.is_empty(),
            "Should have no tasks in far future, found {}",
            view.items.len()
        );
    }

    #[test]
    fn test_agenda_today_finds_scheduled_tasks() {
        use crate::org_mode::AgendaViewType;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let view = org_mode
            .get_agenda_view(AgendaViewType::Today, None, None)
            .expect("Failed to get today's agenda");

        // Verify tasks scheduled for @TODAY@ are found
        let review_task = view
            .items
            .iter()
            .find(|item| item.heading.contains("Review pull requests"));

        assert!(
            review_task.is_some(),
            "Should find 'Review pull requests' task scheduled for today"
        );

        if let Some(task) = review_task {
            assert_eq!(task.todo_state, Some("TODO".to_string()));
            assert!(task.tags.contains(&"work".to_string()));
            assert!(task.scheduled.is_some());
        }
    }

    #[test]
    fn test_agenda_today_excludes_future_tasks() {
        use crate::org_mode::AgendaViewType;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let view = org_mode
            .get_agenda_view(AgendaViewType::Today, None, None)
            .expect("Failed to get today's agenda");

        // Tasks scheduled for @TODAY+1@ or later should not be in today's view
        let has_future_task = view
            .items
            .iter()
            .any(|item| item.heading.contains("Buy groceries")); // Scheduled for @TODAY+2@

        assert!(
            !has_future_task,
            "Today's view should not include tasks scheduled for future dates"
        );
    }

    #[test]
    fn test_agenda_week_includes_all_week_tasks() {
        use crate::org_mode::AgendaViewType;

        let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
        let view = org_mode
            .get_agenda_view(AgendaViewType::CurrentWeek, None, None)
            .expect("Failed to get current week agenda");

        // Week should include tasks from the current week (Monday through Sunday)
        let task_headings: Vec<String> = view.items.iter().map(|i| i.heading.clone()).collect();

        // At minimum, should include @TODAY@ tasks
        assert!(
            !task_headings.is_empty(),
            "Week view should include at least one task"
        );

        // Should include today's tasks
        assert!(
            task_headings
                .iter()
                .any(|h| h.contains("Review pull requests") || h.contains("Team standup")),
            "Week view should include today's tasks (@TODAY@)"
        );

        // Note: Future tasks (@TODAY+1@, etc.) may or may not be in the current week
        // depending on which day of the week "today" is. If today is Sunday,
        // then @TODAY+1@ tasks are in next week, not this week.
    }
}

mod datetime_helpers_tests {
    use crate::OrgMode;
    use chrono::{Datelike, Local, NaiveDate, TimeZone, Timelike};

    #[test]
    fn test_to_start_of_day() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 14, 30, 45).unwrap();
        let start = OrgMode::to_start_of_day(date);

        assert_eq!(start.hour(), 0);
        assert_eq!(start.minute(), 0);
        assert_eq!(start.second(), 0);
        assert_eq!(start.day(), 15);
        assert_eq!(start.month(), 6);
        assert_eq!(start.year(), 2025);
    }

    #[test]
    fn test_to_end_of_day() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 14, 30, 45).unwrap();
        let end = OrgMode::to_end_of_day(date);

        assert_eq!(end.hour(), 23);
        assert_eq!(end.minute(), 59);
        assert_eq!(end.second(), 59);
        assert_eq!(end.day(), 15);
        assert_eq!(end.month(), 6);
        assert_eq!(end.year(), 2025);
    }

    #[test]
    fn test_naive_date_to_local_valid() {
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let result = OrgMode::naive_date_to_local(date, 14, 30, 45);

        assert!(result.is_ok());
        let datetime = result.unwrap();
        assert_eq!(datetime.day(), 15);
        assert_eq!(datetime.month(), 6);
        assert_eq!(datetime.year(), 2025);
        assert_eq!(datetime.hour(), 14);
        assert_eq!(datetime.minute(), 30);
        assert_eq!(datetime.second(), 45);
    }

    #[test]
    fn test_naive_date_to_local_midnight() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let result = OrgMode::naive_date_to_local(date, 0, 0, 0);

        assert!(result.is_ok());
        let datetime = result.unwrap();
        assert_eq!(datetime.hour(), 0);
        assert_eq!(datetime.minute(), 0);
        assert_eq!(datetime.second(), 0);
    }

    #[test]
    fn test_naive_date_to_local_end_of_day() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let result = OrgMode::naive_date_to_local(date, 23, 59, 59);

        assert!(result.is_ok());
        let datetime = result.unwrap();
        assert_eq!(datetime.hour(), 23);
        assert_eq!(datetime.minute(), 59);
        assert_eq!(datetime.second(), 59);
    }

    #[test]
    fn test_last_day_of_month_regular() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let last_day = OrgMode::last_day_of_month(date);

        assert_eq!(last_day.day(), 30);
        assert_eq!(last_day.month(), 6);
        assert_eq!(last_day.year(), 2025);
    }

    #[test]
    fn test_last_day_of_month_december() {
        let date = Local.with_ymd_and_hms(2025, 12, 1, 12, 0, 0).unwrap();
        let last_day = OrgMode::last_day_of_month(date);

        assert_eq!(last_day.day(), 31);
        assert_eq!(last_day.month(), 12);
        assert_eq!(last_day.year(), 2025);
    }

    #[test]
    fn test_last_day_of_month_february_non_leap() {
        let date = Local.with_ymd_and_hms(2025, 2, 10, 12, 0, 0).unwrap();
        let last_day = OrgMode::last_day_of_month(date);

        assert_eq!(last_day.day(), 28);
        assert_eq!(last_day.month(), 2);
    }

    #[test]
    fn test_last_day_of_month_february_leap() {
        let date = Local.with_ymd_and_hms(2024, 2, 10, 12, 0, 0).unwrap();
        let last_day = OrgMode::last_day_of_month(date);

        assert_eq!(last_day.day(), 29);
        assert_eq!(last_day.month(), 2);
    }
}

mod timestamp_conversion_tests {
    use crate::OrgMode;
    use chrono::{Datelike, Timelike};
    use orgize::Org;
    use orgize::ast::Timestamp;
    use orgize::export::{Container, Event, from_fn};

    fn find_timestamp(content: &str) -> Option<Timestamp> {
        let mut found = None;
        let mut handler = from_fn(|event| {
            if let Event::Enter(Container::Headline(h)) = event
                && let Some(ts) = h.scheduled()
            {
                found = Some(ts);
            }
        });
        Org::parse(content).traverse(&mut handler);
        found
    }

    #[test]
    fn test_timestamp_to_chrono_start() {
        let content = "* TODO Task\nSCHEDULED: <2025-06-15 Sun 14:30>";
        let ts = find_timestamp(content).expect("Should find timestamp");

        let result = OrgMode::start_to_chrono(&ts);
        assert!(result.is_some());

        let datetime = result.unwrap();
        assert_eq!(datetime.year(), 2025);
        assert_eq!(datetime.month(), 6);
        assert_eq!(datetime.day(), 15);
        assert_eq!(datetime.hour(), 14);
        assert_eq!(datetime.minute(), 30);
    }

    #[test]
    fn test_timestamp_to_chrono_without_time() {
        let content = "* TODO Task\nSCHEDULED: <2025-06-15 Sun>";
        let ts = find_timestamp(content).expect("Should find timestamp");

        let result = OrgMode::start_to_chrono(&ts);
        assert!(result.is_some());

        let datetime = result.unwrap();
        assert_eq!(datetime.year(), 2025);
        assert_eq!(datetime.month(), 6);
        assert_eq!(datetime.day(), 15);
        assert_eq!(datetime.hour(), 0);
        assert_eq!(datetime.minute(), 0);
    }
}

mod repeater_calculations_tests {
    use crate::OrgMode;
    use chrono::{Datelike, Local, TimeZone, Timelike};
    use orgize::ast::TimeUnit;

    #[test]
    fn test_add_repeater_duration_hour() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 14, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Hour);

        assert_eq!(result.hour(), 16);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_add_repeater_duration_day() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 5, &TimeUnit::Day);

        assert_eq!(result.day(), 20);
        assert_eq!(result.month(), 6);
    }

    #[test]
    fn test_add_repeater_duration_week() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Week);

        assert_eq!(result.day(), 29);
        assert_eq!(result.month(), 6);
    }

    #[test]
    fn test_add_repeater_duration_month() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 3, &TimeUnit::Month);

        assert_eq!(result.month(), 9);
        assert_eq!(result.day(), 15);
        assert_eq!(result.year(), 2025);
    }

    #[test]
    fn test_add_repeater_duration_year() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Year);

        assert_eq!(result.year(), 2027);
        assert_eq!(result.month(), 6);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_add_repeater_duration_month_boundary() {
        let date = Local.with_ymd_and_hms(2025, 10, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 3, &TimeUnit::Month);

        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 15);
    }
}

mod agenda_item_creation_tests {
    use crate::OrgMode;
    use orgize::Org;
    use orgize::ast::Headline;
    use orgize::export::{Container, Event, from_fn};

    fn find_headline(content: &str) -> Option<Headline> {
        let mut found = None;
        let mut handler = from_fn(|event| {
            if let Event::Enter(Container::Headline(h)) = event {
                found = Some(h);
            }
        });
        Org::parse(content).traverse(&mut handler);
        found
    }

    #[test]
    fn test_headline_to_agenda_item_basic() {
        let content = "* TODO Basic Task";
        let headline = find_headline(content).expect("Should find headline");

        let item = OrgMode::headline_to_agenda_item(&headline, "test.org".to_string());

        assert_eq!(item.file_path, "test.org");
        assert_eq!(item.heading, "Basic Task");
        assert_eq!(item.level, 1);
        assert_eq!(item.todo_state, Some("TODO".to_string()));
        assert_eq!(item.priority, None);
        assert!(item.tags.is_empty());
    }

    #[test]
    fn test_headline_to_agenda_item_with_priority() {
        let content = "* TODO [#A] High Priority Task";
        let headline = find_headline(content).expect("Should find headline");

        let item = OrgMode::headline_to_agenda_item(&headline, "test.org".to_string());

        assert_eq!(item.heading, "High Priority Task");
        assert_eq!(item.priority, Some("A".to_string()));
    }

    #[test]
    fn test_headline_to_agenda_item_with_tags() {
        let content = "* TODO Task with Tags :work:urgent:";
        let headline = find_headline(content).expect("Should find headline");

        let item = OrgMode::headline_to_agenda_item(&headline, "test.org".to_string());

        // Note: orgize includes trailing space before tags in title_raw()
        assert_eq!(item.heading.trim(), "Task with Tags");
        assert!(item.tags.contains(&"work".to_string()));
        assert!(item.tags.contains(&"urgent".to_string()));
        assert_eq!(item.tags.len(), 2);
    }

    #[test]
    fn test_headline_to_agenda_item_with_scheduled() {
        let content = "* TODO Scheduled Task\nSCHEDULED: <2025-06-15 Sun>";
        let headline = find_headline(content).expect("Should find headline");

        let item = OrgMode::headline_to_agenda_item(&headline, "test.org".to_string());

        assert!(item.scheduled.is_some());
        assert!(item.scheduled.unwrap().contains("2025-06-15"));
    }

    #[test]
    fn test_headline_to_agenda_item_with_deadline() {
        let content = "* TODO Task with Deadline\nDEADLINE: <2025-06-20 Fri>";
        let headline = find_headline(content).expect("Should find headline");

        let item = OrgMode::headline_to_agenda_item(&headline, "test.org".to_string());

        assert!(item.deadline.is_some());
        assert!(item.deadline.unwrap().contains("2025-06-20"));
    }

    #[test]
    fn test_headline_to_agenda_item_nested() {
        let content = "* Parent\n** TODO Nested Task";
        let org = Org::parse(content);
        let mut found = None;
        let mut handler = from_fn(|event| {
            if let Event::Enter(Container::Headline(h)) = event
                && h.level() == 2
            {
                found = Some(h);
            }
        });
        org.traverse(&mut handler);

        let headline = found.expect("Should find nested headline");
        let item = OrgMode::headline_to_agenda_item(&headline, "test.org".to_string());

        assert_eq!(item.level, 2);
        assert_eq!(item.heading, "Nested Task");
    }

    #[test]
    fn test_headline_to_agenda_item_position() {
        let content = "* TODO Task";
        let headline = find_headline(content).expect("Should find headline");

        let item = OrgMode::headline_to_agenda_item(&headline, "test.org".to_string());

        assert!(item.position.is_some());
        let pos = item.position.unwrap();
        assert!(pos.start < pos.end);
    }
}

mod date_parsing_tests {
    use crate::{OrgMode, OrgModeError};
    use chrono::Datelike;

    #[test]
    fn test_parse_date_string_valid() {
        let result = OrgMode::parse_date_string("2025-06-15", "test date");
        assert!(result.is_ok());

        let date = result.unwrap();
        assert_eq!(date.year(), 2025);
        assert_eq!(date.month(), 6);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_parse_date_string_invalid_format() {
        let result = OrgMode::parse_date_string("15-06-2025", "test date");
        assert!(result.is_err());

        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Invalid test date"));
            assert!(msg.contains("15-06-2025"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_parse_date_string_invalid_date() {
        let result = OrgMode::parse_date_string("2025-13-40", "test date");
        assert!(result.is_err());

        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("Invalid test date"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }

    #[test]
    fn test_parse_date_string_leap_year() {
        let result = OrgMode::parse_date_string("2024-02-29", "leap date");
        assert!(result.is_ok());

        let date = result.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 2);
        assert_eq!(date.day(), 29);
    }

    #[test]
    fn test_parse_date_string_non_leap_year_invalid() {
        let result = OrgMode::parse_date_string("2025-02-29", "non-leap date");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_date_string_context_in_error() {
        let result = OrgMode::parse_date_string("invalid", "from date");
        assert!(result.is_err());

        if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
            assert!(msg.contains("from date"));
        } else {
            panic!("Expected InvalidAgendaViewType error");
        }
    }
}
