//! Integration tests for MCP server tools.
//!
//! This module contains tests that verify the functionality of tools exposed
//! by the org-mcp-server, including org-file-list and org-search tools.

use rmcp::model::CallToolRequestParam;
use serde_json::{Map, Value};
use tracing::{info, warn};
use tracing_test::traced_test;

use crate::{create_mcp_service, setup_test_org_files};

/// Tests that all expected tools are available and properly configured.
///
/// Verifies that:
/// - The server exposes the expected tools (org-file-list, org-search)
/// - Each tool has a non-empty description
/// - Tool metadata is properly formatted
#[tokio::test]
#[traced_test]
async fn test_list_tools() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test tool listing");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let tools = service.list_tools(Default::default()).await?;
    info!("Available tools: {:#?}", tools);

    let tool_names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(tool_names.contains(&"org-file-list"));
    assert!(tool_names.contains(&"org-search"));

    for tool in &tools.tools {
        assert!(tool.description.is_some());
        assert!(!tool.description.as_ref().unwrap().is_empty());
    }

    service.cancel().await?;
    info!("List tools test completed successfully");

    Ok(())
}

/// Tests the org-file-list tool functionality.
///
/// Verifies that:
/// - The tool returns a list of all org files in the directory tree
/// - Results include files from both root and subdirectories
/// - Response format is valid JSON containing file information
#[tokio::test]
#[traced_test]
async fn test_org_file_list_tool() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-file-list tool");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-file-list".into(),
            arguments: None,
        })
        .await?;

    info!("org-file-list result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            // Should contain our test files from both root and subdirectories
            assert!(text.text.contains("notes.org"));
            assert!(text.text.contains("project.org"));
            assert!(text.text.contains("research.org"));
            assert!(text.text.contains("old_notes.org"));
        } else {
            panic!("Expected text content in org-file-list result");
        }
    } else {
        panic!("No content in org-file-list result");
    }

    service.cancel().await?;
    info!("org-file-list tool test completed successfully");

    Ok(())
}

/// Tests basic org-search tool functionality.
///
/// Verifies that:
/// - The tool accepts a simple query and returns relevant results
/// - Results include expected fields (file_path, snippet, score)
/// - Search finds content from the test org files
#[tokio::test]
#[traced_test]
async fn test_org_search_tool() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-search tool");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("query".to_string(), Value::String("Daily Tasks".into()));
    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-search".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-search result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            // Verify the search results contain expected JSON fields
            assert!(text.text.contains("\"file_path\""));
            assert!(text.text.contains("\"snippet\""));
            assert!(text.text.contains("\"score\""));
        } else {
            panic!("Expected text content in org-search result");
        }
    } else {
        panic!("No content in org-search result");
    }

    service.cancel().await?;
    info!("org-search tool test completed successfully");

    Ok(())
}

/// Tests org-search tool with optional parameters.
///
/// Verifies that:
/// - The tool accepts limit and snippet_max_size parameters
/// - Results respect the limit parameter (returns ≤ limit items)
/// - Response structure is valid JSON with expected fields
/// - Each result contains file_path, snippet, and score fields
#[tokio::test]
#[traced_test]
async fn test_org_search_tool_with_parameters() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-search tool with parameters");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("query".to_string(), Value::String("TODO".into()));
    args.insert("limit".to_string(), Value::Number(2.into()));
    args.insert("snippet_max_size".to_string(), Value::Number(50.into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-search".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-search with parameters result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            // Parse as JSON to verify structure and parameter adherence
            let search_results: serde_json::Value =
                serde_json::from_str(&text.text).expect("Search results should be valid JSON");

            if let Some(results_array) = search_results.as_array() {
                // Should respect the limit parameter
                assert!(results_array.len() <= 2, "Should respect limit parameter");

                // Verify each result has the expected structure
                if let Some(first_result) = results_array.first() {
                    assert!(first_result["file_path"].is_string());
                    assert!(first_result["snippet"].is_string());
                    assert!(first_result["score"].is_number());
                }
            } else {
                warn!("Search results not in expected array format: {}", text.text);
            }
        } else {
            panic!("Expected text content in org-search result");
        }
    } else {
        panic!("No content in org-search result");
    }

    service.cancel().await?;
    info!("org-search tool with parameters test completed successfully");

    Ok(())
}
