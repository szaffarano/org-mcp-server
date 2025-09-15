//! Integration tests for MCP server resources.
//!
//! This module contains tests that verify the functionality of resources exposed
//! by the org-mcp-server, including directory listing, file content access,
//! outline structures, heading access, and ID-based lookups.

use rmcp::model::ReadResourceRequestParam;
use tracing::info;
use tracing_test::traced_test;

use crate::{create_mcp_service, setup_test_org_files};

/// Tests that resource listing functionality works correctly.
///
/// Verifies that:
/// - The server lists available base resources
/// - The org:// directory listing resource is present
/// - Resource metadata includes name, description, and MIME type
#[tokio::test]
#[traced_test]
async fn test_list_resources() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test resource listing");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // List available resources
    let resources = service.list_resources(Default::default()).await?;
    info!("Available resources: {:#?}", resources);

    // Verify we have the expected base resource
    assert!(!resources.resources.is_empty());

    // Check for the org:// directory listing resource
    let org_resource = resources
        .resources
        .iter()
        .find(|r| r.uri == "org://")
        .expect("Should have org:// resource");

    assert_eq!(org_resource.name, "org");
    assert!(org_resource.description.is_some());
    assert!(org_resource.mime_type.is_some());

    service.cancel().await?;
    info!("List resources test completed successfully");

    Ok(())
}

/// Tests that resource template listing functionality works correctly.
///
/// Verifies that:
/// - All expected resource templates are available
/// - Templates include org://{file}, org-outline://{file}, etc.
/// - Each template has proper metadata (name, description, MIME type)
#[tokio::test]
#[traced_test]
async fn test_list_resource_templates() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test resource templates listing");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // List available resource templates
    let templates = service.list_resource_templates(Default::default()).await?;
    info!("Available resource templates: {:#?}", templates);

    // Verify we have the expected resource templates
    assert!(!templates.resource_templates.is_empty());

    let template_uris: Vec<&str> = templates
        .resource_templates
        .iter()
        .map(|t| t.uri_template.as_ref())
        .collect();

    // Check that all expected template URIs are present
    assert!(template_uris.contains(&"org://{file}"));
    assert!(template_uris.contains(&"org-outline://{file}"));
    assert!(template_uris.contains(&"org-heading://{file}#{heading}"));
    assert!(template_uris.contains(&"org-id://{id}"));

    // Verify each template has required metadata fields
    for template in &templates.resource_templates {
        assert!(!template.name.is_empty());
        assert!(template.description.is_some());
        assert!(template.mime_type.is_some());
    }

    service.cancel().await?;
    info!("Resource templates test completed successfully");

    Ok(())
}

/// Tests org:// directory listing resource functionality.
///
/// Verifies that:
/// - The org:// resource returns directory listing information
/// - Listing includes all test org files from all subdirectories
/// - Content is properly formatted and accessible
#[tokio::test]
#[traced_test]
async fn test_read_org_directory_resource() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org directory resource reading");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // Read the org:// directory listing resource
    let result = service
        .read_resource(ReadResourceRequestParam {
            uri: "org://".to_string(),
        })
        .await?;

    info!("Directory resource result: {:#?}", result);
    assert!(!result.contents.is_empty());

    // Verify the response contains file listing information
    if let Some(content) = result.contents.first() {
        if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
            // Should contain our test files from both root and subdirectories
            assert!(text.contains("notes.org"));
            assert!(text.contains("project.org"));
            assert!(text.contains("research.org"));
            assert!(text.contains("old_notes.org"));
        } else {
            panic!("Expected text content in directory listing result");
        }
    } else {
        panic!("No content in directory listing result");
    }

    service.cancel().await?;
    info!("Directory resource test completed successfully");

    Ok(())
}

/// Tests org://{file} file content resource functionality.
///
/// Verifies that:
/// - Individual org files can be read via org:// URI scheme
/// - File content is returned accurately and completely
/// - Org-mode structure and metadata are preserved
#[tokio::test]
#[traced_test]
async fn test_read_org_file_resource() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org file resource reading");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // Read a specific org file resource
    let result = service
        .read_resource(ReadResourceRequestParam {
            uri: "org://notes.org".to_string(),
        })
        .await?;

    info!("File resource result: {:#?}", result);
    assert!(!result.contents.is_empty());

    // Verify the response contains the actual file content
    if let Some(content) = result.contents.first() {
        if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
            // Should contain the content from our notes.org file
            assert!(text.contains("#+TITLE: Notes"));
            assert!(text.contains("* Daily Tasks"));
            assert!(text.contains("** TODO Buy groceries"));
            assert!(text.contains("** DONE Read book"));
            assert!(text.contains(":ID: daily-tasks-123"));
        } else {
            panic!("Expected text content in file reading result");
        }
    } else {
        panic!("No content in file reading result");
    }

    service.cancel().await?;
    info!("File resource test completed successfully");

    Ok(())
}

/// Tests org-outline://{file} outline structure resource functionality.
///
/// Verifies that:
/// - Outline resources return structured hierarchical data
/// - Response is valid JSON format
/// - Outline contains expected structural information
#[tokio::test]
#[traced_test]
async fn test_read_org_outline_resource() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org outline resource reading");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // Read the outline structure of a specific org file
    let result = service
        .read_resource(ReadResourceRequestParam {
            uri: "org-outline://project.org".to_string(),
        })
        .await?;

    info!("Outline resource result: {:#?}", result);
    assert!(!result.contents.is_empty());

    // Verify the response contains outline structure
    if let Some(content) = result.contents.first() {
        if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
            // Should be JSON format containing the outline structure
            let outline_json: serde_json::Value =
                serde_json::from_str(text).expect("Outline should be valid JSON");

            // Check that we have a structured outline
            assert!(outline_json.is_object() || outline_json.is_array());

            // The response should contain information about our project.org structure
            let outline_str = text.to_lowercase();
            assert!(
                outline_str.contains("backend")
                    || outline_str.contains("frontend")
                    || outline_str.contains("development")
            );
        } else {
            panic!("Expected text content in outline reading result");
        }
    } else {
        panic!("No content in outline reading result");
    }

    service.cancel().await?;
    info!("Outline resource test completed successfully");

    Ok(())
}

/// Tests org-heading://{file}#{heading} specific heading resource functionality.
///
/// Verifies that:
/// - Specific headings can be accessed by path
/// - Content includes heading text and associated content
/// - Properties and metadata are included in the response
#[tokio::test]
#[traced_test]
async fn test_read_org_heading_resource() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org heading resource reading");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // Read a specific heading from an org file
    let result = service
        .read_resource(ReadResourceRequestParam {
            uri: "org-heading://notes.org#Daily Tasks".to_string(),
        })
        .await?;

    info!("Heading resource result: {:#?}", result);
    assert!(!result.contents.is_empty());

    // Verify the response contains the specific heading content
    if let Some(content) = result.contents.first() {
        if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
            // Should contain content from the "Daily Tasks" heading
            assert!(text.contains("Daily Tasks"));
            assert!(text.contains("TODO Buy groceries") || text.contains("DONE Read book"));

            // Should contain the ID property from the heading
            assert!(text.contains("daily-tasks-123"));
        } else {
            panic!("Expected text content in heading reading result");
        }
    } else {
        panic!("No content in heading reading result");
    }

    service.cancel().await?;
    info!("Heading resource test completed successfully");

    Ok(())
}

/// Tests org-id://{id} ID-based content resource functionality.
///
/// Verifies that:
/// - Content can be accessed by org-mode ID property
/// - Response includes the element and its content
/// - ID lookup works across different files
#[tokio::test]
#[traced_test]
async fn test_read_org_id_resource() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org ID resource reading");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // Read content by ID property
    let result = service
        .read_resource(ReadResourceRequestParam {
            uri: "org-id://daily-tasks-123".to_string(),
        })
        .await?;

    info!("ID resource result: {:#?}", result);
    assert!(!result.contents.is_empty());

    // Verify the response contains the content with the specified ID
    if let Some(content) = result.contents.first() {
        if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
            // Should contain content from the element with ID daily-tasks-123
            assert!(text.contains("Daily Tasks"));
            assert!(text.contains(":ID: daily-tasks-123"));
        } else {
            panic!("Expected text content in ID reading result");
        }
    } else {
        panic!("No content in ID reading result");
    }

    service.cancel().await?;
    info!("ID resource test completed successfully");

    Ok(())
}

/// Tests error handling for invalid resource URIs.
///
/// Verifies that:
/// - Invalid URI schemes are properly rejected
/// - Nonexistent files/headings/IDs return appropriate errors
/// - Malformed URIs are handled gracefully
/// - Error responses are informative and consistent
#[tokio::test]
#[traced_test]
async fn test_invalid_resource_uris() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test invalid resource URI handling");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // Test various invalid resource URIs
    let invalid_uris = vec![
        "invalid://path",
        "org://nonexistent.org",
        "org-outline://nonexistent.org",
        "org-heading://notes.org#NonexistentHeading",
        "org-id://nonexistent-id",
        "",
        "not-a-resource",
        "org-heading://notes.org", // Missing heading part
        "org-heading://#heading",  // Missing file part
    ];

    for invalid_uri in invalid_uris {
        info!("Testing invalid URI: {}", invalid_uri);

        let result = service
            .read_resource(ReadResourceRequestParam {
                uri: invalid_uri.to_string(),
            })
            .await;

        // Should get an error for invalid URIs
        match result {
            Err(_) => {
                info!("Correctly received error for invalid URI: {}", invalid_uri);
            }
            Ok(_) => {
                panic!("Expected error for invalid URI: {}", invalid_uri);
            }
        }
    }

    service.cancel().await?;
    info!("Invalid resource URI test completed successfully");

    Ok(())
}
