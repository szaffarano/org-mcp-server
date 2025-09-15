use std::{
    env,
    path::{self, PathBuf},
};

use rmcp::{
    model::{CallToolRequestParam, ReadResourceRequestParam},
    transport::ConfigureCommandExt,
};
use serde_json::{Map, Value};
use std::fs;
use tempfile::TempDir;
use tokio::process::Command;
use tracing::{error, info};
use tracing_test::traced_test;

#[macro_export]
macro_rules! create_mcp_service {
    ($temp_dir:expr) => {{
        use rmcp::{
            ServiceExt,
            transport::{ConfigureCommandExt, TokioChildProcess},
        };
        use tracing::error;

        let mut command = tokio::process::Command::new(get_binary_path("org-mcp-server"))
            .configure(|cmd| {
                cmd.args(["--root", $temp_dir.path().to_str().unwrap()]);
            });

        with_coverage_env(&mut command);

        ().serve(TokioChildProcess::new(command)?)
            .await
            .map_err(|e| {
                error!("Failed to connect to server: {}", e);
                e
            })?
    }};
}

pub fn get_binary_path(name: &str) -> PathBuf {
    let env_var = format!("CARGO_BIN_EXE_{name}");
    env::var_os(env_var)
        .map(|p| p.into())
        .unwrap_or_else(|| target_dir().join(format!("{}{}", name, env::consts::EXE_SUFFIX)))
}

pub fn with_coverage_env(cmd: &mut Command) {
    for (key, value) in std::env::vars() {
        if key.contains("LLVM") {
            cmd.env(&key, &value);
        }
    }
}

fn target_dir() -> path::PathBuf {
    env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        })
        .expect("this should only be used where a `current_exe` can be set")
}

fn setup_test_org_files() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    fs::write(
        temp_dir.path().join("notes.org"),
        r#"#+TITLE: Notes
#+DATE: 2024-01-01

* Daily Tasks
  :PROPERTIES:
  :ID: daily-tasks-123
  :END:
** TODO Buy groceries
   :PROPERTIES:
   :ID: task-groceries-456
   :END:
** DONE Read book
   :PROPERTIES:
   :ID: task-book-789
   :END:
   - Author: John Doe
   - Pages: 256

* Project Ideas
** Web scraper for news articles
** Mobile app for habit tracking

* Meeting Notes
** 2024-01-15 Team Standup
   - Discussed sprint goals
   - Reviewed backlog items
   - Assigned new tasks

* Random Thoughts
This is some random text with keywords like productivity, efficiency, and automation.
"#,
    )?;
    fs::write(
        temp_dir.path().join("project.org"),
        r#"#+TITLE: Project Planning
#+AUTHOR: Test User
#+DATE: 2024-01-02

* Backend Development
  :PROPERTIES:
  :ID: backend-dev-101
  :END:
** API Design
   :PROPERTIES:
   :ID: api-design-102
   :END:
   - REST endpoints
   - Authentication
   - Database schema

** Implementation
*** TODO Set up development environment
*** TODO Create user authentication
*** DONE Initialize project structure

* Frontend Development
  :PROPERTIES:
  :ID: frontend-dev-201
  :END:
** UI/UX Design
   - Wireframes
   - User flow
   - Color scheme

** React Components
*** TODO Header component
*** TODO Navigation menu
*** TODO User dashboard

* Testing Strategy
** Unit Tests
   - Controller tests
   - Service layer tests
   - Database tests

** Integration Tests
   - API endpoint tests
   - End-to-end tests
"#,
    )?;
    fs::write(
        temp_dir.path().join("research.org"),
        r#"#+TITLE: Research Notes
#+TAGS: research, technology, AI

* Machine Learning
  :PROPERTIES:
  :ID: ml-research-301
  :END:
** Deep Learning Fundamentals
   :PROPERTIES:
   :ID: dl-fundamentals-302
   :END:
   - Neural networks
   - Backpropagation
   - Gradient descent

** Natural Language Processing
   - Tokenization
   - Word embeddings
   - Transformer models

* Rust Programming
  :PROPERTIES:
  :ID: rust-programming-401
  :END:
** Memory Management
   - Ownership
   - Borrowing
   - Lifetimes

** Async Programming
   - Futures
   - Tokio runtime
   - async/await syntax

* Tools and Technologies
** Development Tools
   - Git version control
   - Docker containers
   - CI/CD pipelines

** Databases
   - PostgreSQL
   - Redis
   - MongoDB
"#,
    )?;

    let subdir = temp_dir.path().join("archive");
    fs::create_dir(&subdir)?;
    fs::write(
        subdir.join("old_notes.org"),
        r#"#+TITLE: Old Notes
#+DATE: 2023-12-01

* Archived Ideas
  :PROPERTIES:
  :ID: archived-ideas-501
  :END:
** Idea 1: Mobile game development
** Idea 2: Productivity app
** Idea 3: E-commerce platform

* Completed Projects
** Personal website
   - Built with React
   - Deployed on Netlify
   - Source code on GitHub

* Learning Resources
** Books
   - The Pragmatic Programmer
   - Clean Code
   - Design Patterns

** Online Courses
   - Rust Programming Course
   - React Advanced Patterns
   - Database Design Fundamentals
"#,
    )?;

    Ok(temp_dir)
}

#[traced_test]
#[tokio::test]
async fn test_graceful_close_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP server using pre-compiled binary");

    let org_dir = setup_test_org_files()?;
    let binary = get_binary_path("org-mcp-server");
    let mut command = Command::new(binary).configure(|cmd| {
        cmd.args(["--root", org_dir.path().to_str().unwrap()]);
    });
    with_coverage_env(&mut command);

    let mut child = command.stdin(std::process::Stdio::piped()).spawn()?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    match child.try_wait()? {
        Some(status) => {
            error!("MCP server exited prematurely with status: {}", status);
            return Err("MCP server exited prematurely".into());
        }
        None => {
            info!("MCP server is running");
        }
    }

    if let Some(stdin) = child.stdin.take() {
        drop(stdin);
    }

    child.wait().await?;
    info!("MCP server process terminated");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_mcp_server_connection() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test nvim-mcp server");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    let server_info = service.peer_info();
    info!("Connected to server: {:#?}", server_info);

    if let Some(info) = server_info {
        assert!(info.instructions.is_some());
        assert!(info.capabilities.tools.is_some());
    } else {
        panic!("Expected server info to be present");
    }

    service.cancel().await?;
    info!("MCP server connection test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_list_tools() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test org-mcp-server");

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
            // Should contain our test files
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
            // Parse as JSON to verify structure
            let search_results: serde_json::Value =
                serde_json::from_str(&text.text).expect("Search results should be valid JSON");

            if let Some(results_array) = search_results.as_array() {
                // Should respect the limit parameter
                assert!(results_array.len() <= 2, "Should respect limit parameter");

                if let Some(first_result) = results_array.first() {
                    assert!(first_result["file_path"].is_string());
                    assert!(first_result["snippet"].is_string());
                    assert!(first_result["score"].is_number());
                }
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
            // Should contain our test files
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

    assert!(template_uris.contains(&"org://{file}"));
    assert!(template_uris.contains(&"org-outline://{file}"));
    assert!(template_uris.contains(&"org-heading://{file}#{heading}"));
    assert!(template_uris.contains(&"org-id://{id}"));

    // Verify each template has required fields
    for template in &templates.resource_templates {
        assert!(!template.name.is_empty());
        assert!(template.description.is_some());
        assert!(template.mime_type.is_some());
    }

    service.cancel().await?;
    info!("Resource templates test completed successfully");

    Ok(())
}

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
