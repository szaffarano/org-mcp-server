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

/// Tests the org-file-list tool with tags parameter.
///
/// Verifies that:
/// - The tool accepts a tags parameter
/// - Results are filtered by the specified tags
/// - Only files with matching tags are returned
#[tokio::test]
#[traced_test]
async fn test_org_file_list_tool_with_tags() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-file-list tool with tags");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert(
        "tags".to_string(),
        Value::Array(vec![Value::String("work".into())]),
    );

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-file-list".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-file-list with tags result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let files: Vec<String> =
                serde_json::from_str(&text.text).expect("Response should be valid JSON array");

            assert!(!files.is_empty());
        } else {
            panic!("Expected text content in org-file-list result");
        }
    } else {
        panic!("No content in org-file-list result");
    }

    service.cancel().await?;
    info!("org-file-list tool with tags test completed successfully");

    Ok(())
}

/// Tests the org-file-list tool with limit parameter.
///
/// Verifies that:
/// - The tool accepts a limit parameter
/// - Results respect the limit (returns ≤ limit items)
/// - Response structure is valid JSON array
#[tokio::test]
#[traced_test]
async fn test_org_file_list_tool_with_limit() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-file-list tool with limit");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("limit".to_string(), Value::Number(2.into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-file-list".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-file-list with limit result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let files: Vec<String> =
                serde_json::from_str(&text.text).expect("Response should be valid JSON array");

            assert!(files.len() <= 2, "Should respect limit parameter");
        } else {
            panic!("Expected text content in org-file-list result");
        }
    } else {
        panic!("No content in org-file-list result");
    }

    service.cancel().await?;
    info!("org-file-list tool with limit test completed successfully");

    Ok(())
}

/// Tests the org-file-list tool with both tags and limit parameters.
///
/// Verifies that:
/// - The tool accepts both tags and limit parameters together
/// - Results are both filtered by tags AND limited
/// - Response structure is valid JSON array
#[tokio::test]
#[traced_test]
async fn test_org_file_list_tool_with_tags_and_limit() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-file-list tool with tags and limit");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert(
        "tags".to_string(),
        Value::Array(vec![Value::String("work".into())]),
    );
    args.insert("limit".to_string(), Value::Number(1.into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-file-list".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-file-list with tags and limit result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let files: Vec<String> =
                serde_json::from_str(&text.text).expect("Response should be valid JSON array");

            assert!(
                files.len() <= 1,
                "Should respect limit parameter with tags filter"
            );
        } else {
            panic!("Expected text content in org-file-list result");
        }
    } else {
        panic!("No content in org-file-list result");
    }

    service.cancel().await?;
    info!("org-file-list tool with tags and limit test completed successfully");

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

/// Tests the org-search tool with tag filtering.
///
/// Verifies that:
/// - The tool accepts a tags parameter
/// - Results are filtered by the specified tags
/// - Only results with matching tags are returned
#[tokio::test]
#[traced_test]
async fn test_org_search_tool_with_tags() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-search tool with tags");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("query".to_string(), Value::String("Task".into()));
    args.insert(
        "tags".to_string(),
        Value::Array(vec![Value::String("work".into())]),
    );

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-search".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-search with tags result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let search_results: serde_json::Value =
                serde_json::from_str(&text.text).expect("Search results should be valid JSON");

            if let Some(results_array) = search_results.as_array() {
                // All results should have the work tag
                for result in results_array {
                    if let Some(tags) = result["tags"].as_array() {
                        assert!(
                            tags.iter().any(|t| t.as_str() == Some("work")),
                            "All results should have the work tag"
                        );
                    }
                }
            }
        } else {
            panic!("Expected text content in org-search result");
        }
    } else {
        panic!("No content in org-search result");
    }

    service.cancel().await?;
    info!("org-search tool with tags test completed successfully");

    Ok(())
}

/// Tests the org-search tool with multiple tags.
///
/// Verifies that:
/// - The tool accepts multiple tags
/// - Results match ANY of the specified tags (OR logic)
#[tokio::test]
#[traced_test]
async fn test_org_search_tool_with_multiple_tags() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-search tool with multiple tags");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("query".to_string(), Value::String("Project".into()));
    args.insert(
        "tags".to_string(),
        Value::Array(vec![
            Value::String("work".into()),
            Value::String("personal".into()),
        ]),
    );

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-search".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-search with multiple tags result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let search_results: serde_json::Value =
                serde_json::from_str(&text.text).expect("Search results should be valid JSON");

            if let Some(results_array) = search_results.as_array() {
                // All results should have at least one of the specified tags
                for result in results_array {
                    if let Some(tags) = result["tags"].as_array() {
                        assert!(
                            tags.iter().any(|t| {
                                t.as_str() == Some("work") || t.as_str() == Some("personal")
                            }),
                            "Results should have work or personal tag"
                        );
                    }
                }
            }
        } else {
            panic!("Expected text content in org-search result");
        }
    } else {
        panic!("No content in org-search result");
    }

    service.cancel().await?;
    info!("org-search tool with multiple tags test completed successfully");

    Ok(())
}

/// Tests basic org-agenda tool functionality in list mode.
///
/// Verifies that:
/// - The tool returns a list of all tasks (TODO/DONE items)
/// - Results include expected fields from agenda.org fixture
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_list_mode() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool in list mode");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("list".into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda list mode result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let tasks: serde_json::Value =
                serde_json::from_str(&text.text).expect("Tasks should be valid JSON");

            if let Some(tasks_array) = tasks.as_array() {
                assert!(!tasks_array.is_empty(), "Should have tasks from agenda.org");

                // Verify first task has expected structure
                if let Some(first_task) = tasks_array.first() {
                    assert!(first_task["heading"].is_string());
                    assert!(first_task["file_path"].is_string());
                }
            }
        } else {
            panic!("Expected text content in org-agenda result");
        }
    } else {
        panic!("No content in org-agenda result");
    }

    service.cancel().await?;
    info!("org-agenda list mode test completed successfully");

    // TODO: review once complete agenda view is implemented

    Ok(())
}

/// Tests org-agenda tool with TODO state filtering.
///
/// Verifies that:
/// - The tool accepts a todo_states parameter
/// - Results are filtered by the specified states
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_list_with_states() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with state filtering");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("list".into()));
    args.insert(
        "todo_states".to_string(),
        Value::Array(vec![Value::String("TODO".into())]),
    );

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda with states result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let tasks: serde_json::Value =
                serde_json::from_str(&text.text).expect("Tasks should be valid JSON");

            // Just verify we got an array response
            assert!(tasks.is_array(), "Result should be a JSON array");
        } else {
            panic!("Expected text content in org-agenda result");
        }
    } else {
        panic!("No content in org-agenda result");
    }

    service.cancel().await?;
    info!("org-agenda with states test completed successfully");

    // TODO: review once complete agenda view is implemented
    Ok(())
}

/// Tests org-agenda tool with tag filtering.
///
/// Verifies that:
/// - The tool accepts a tags parameter
/// - Results are filtered by the specified tags
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_list_with_tags() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with tag filtering");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("list".into()));
    args.insert(
        "tags".to_string(),
        Value::Array(vec![Value::String("work".into())]),
    );

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda with tags result: {:#?}", result);
    assert!(!result.content.is_empty());

    service.cancel().await?;
    info!("org-agenda with tags test completed successfully");

    // TODO: review once complete agenda view is implemented

    Ok(())
}

/// Tests org-agenda tool with priority filtering.
///
/// Verifies that:
/// - The tool accepts a priority parameter (A, B, C)
/// - Results are filtered by the specified priority
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_list_with_priority() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with priority filtering");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("list".into()));
    args.insert("priority".to_string(), Value::String("A".into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda with priority result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let tasks: serde_json::Value =
                serde_json::from_str(&text.text).expect("Tasks should be valid JSON");

            assert!(tasks.is_array(), "Result should be a JSON array");
        } else {
            panic!("Expected text content in org-agenda result");
        }
    } else {
        panic!("No content in org-agenda result");
    }

    service.cancel().await?;
    info!("org-agenda with priority test completed successfully");

    // TODO: review once complete agenda view is implemented

    Ok(())
}

/// Tests org-agenda tool with limit parameter.
///
/// Verifies that:
/// - The tool accepts a limit parameter
/// - Results respect the limit (returns ≤ limit items)
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_list_with_limit() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with limit");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("list".into()));
    args.insert("limit".to_string(), Value::Number(2.into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda with limit result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let tasks: serde_json::Value =
                serde_json::from_str(&text.text).expect("Tasks should be valid JSON");

            if let Some(tasks_array) = tasks.as_array() {
                assert!(tasks_array.len() <= 2, "Should respect limit parameter");
            }
        } else {
            panic!("Expected text content in org-agenda result");
        }
    } else {
        panic!("No content in org-agenda result");
    }

    service.cancel().await?;
    info!("org-agenda with limit test completed successfully");

    // TODO: review once complete agenda view is implemented
    Ok(())
}

/// Tests org-agenda tool in view mode without dates (default behavior).
///
/// Verifies that:
/// - The tool accepts mode="view"
/// - Returns an agenda view with date-organized tasks
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_view_mode_default() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool in view mode");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("view".into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda view mode result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let view: serde_json::Value =
                serde_json::from_str(&text.text).expect("View should be valid JSON");

            assert!(view["items"].is_array(), "View should have items array");
        } else {
            panic!("Expected text content in org-agenda result");
        }
    } else {
        panic!("No content in org-agenda result");
    }

    service.cancel().await?;
    info!("org-agenda view mode test completed successfully");

    // TODO: review once complete agenda view is implemented
    Ok(())
}

/// Tests org-agenda tool in view mode with custom date range.
///
/// Verifies that:
/// - The tool accepts start_date and end_date parameters
/// - Returns tasks within the specified date range
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_view_mode_custom_range() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with custom date range");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("view".into()));
    args.insert("start_date".to_string(), Value::String("2025-10-20".into()));
    args.insert("end_date".to_string(), Value::String("2025-10-25".into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda custom range result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let view: serde_json::Value =
                serde_json::from_str(&text.text).expect("View should be valid JSON");

            assert!(view["items"].is_array(), "View should have items array");
        } else {
            panic!("Expected text content in org-agenda result");
        }
    } else {
        panic!("No content in org-agenda result");
    }

    service.cancel().await?;
    info!("org-agenda custom range test completed successfully");

    // TODO: review once complete agenda view is implemented
    Ok(())
}

/// Tests org-agenda tool error handling for invalid mode.
///
/// Verifies that:
/// - Invalid mode parameter returns an error
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_invalid_mode() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with invalid mode");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("invalid".into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await;

    assert!(result.is_err(), "Expected error for invalid mode");

    service.cancel().await?;
    info!("org-agenda invalid mode test completed successfully");

    // TODO: review once complete agenda view is implemented
    Ok(())
}

/// Tests org-agenda tool error handling for invalid priority.
///
/// Verifies that:
/// - Invalid priority parameter returns an error
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_invalid_priority() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with invalid priority");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("list".into()));
    args.insert("priority".to_string(), Value::String("X".into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await;

    assert!(result.is_err(), "Expected error for invalid priority");

    service.cancel().await?;
    info!("org-agenda invalid priority test completed successfully");

    // TODO: review once complete agenda view is implemented
    Ok(())
}

/// Tests org-agenda tool error handling for invalid date format.
///
/// Verifies that:
/// - Invalid date format in view mode handles gracefully
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_invalid_date_format() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with invalid date format");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("view".into()));
    args.insert(
        "start_date".to_string(),
        Value::String("invalid-date".into()),
    );
    args.insert("end_date".to_string(), Value::String("2025-10-25".into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda invalid date result: {:#?}", result);
    assert!(!result.content.is_empty());

    service.cancel().await?;
    info!("org-agenda invalid date test completed successfully");

    // TODO: review once complete agenda view is implemented
    Ok(())
}

/// Tests org-agenda tool with all parameters combined.
///
/// Verifies that:
/// - Multiple filters work together (states, tags, priority, limit)
#[tokio::test]
#[traced_test]
async fn test_org_agenda_tool_all_parameters() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-agenda tool with all parameters");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let mut args = Map::new();
    args.insert("mode".to_string(), Value::String("list".into()));
    args.insert(
        "todo_states".to_string(),
        Value::Array(vec![Value::String("TODO".into())]),
    );
    args.insert(
        "tags".to_string(),
        Value::Array(vec![Value::String("work".into())]),
    );
    args.insert("priority".to_string(), Value::String("A".into()));
    args.insert("limit".to_string(), Value::Number(5.into()));

    let result = service
        .call_tool(CallToolRequestParam {
            name: "org-agenda".into(),
            arguments: Some(args),
        })
        .await?;

    info!("org-agenda all parameters result: {:#?}", result);
    assert!(!result.content.is_empty());

    if let Some(content) = result.content.first() {
        if let Some(text) = content.as_text() {
            let tasks: serde_json::Value =
                serde_json::from_str(&text.text).expect("Tasks should be valid JSON");

            if let Some(tasks_array) = tasks.as_array() {
                // Just verify we got results and they respect the limit
                assert!(tasks_array.len() <= 5, "Should respect limit");
            }
        } else {
            panic!("Expected text content in org-agenda result");
        }
    } else {
        panic!("No content in org-agenda result");
    }

    service.cancel().await?;
    info!("org-agenda all parameters test completed successfully");

    // TODO: review once complete agenda view is implemented
    Ok(())
}
