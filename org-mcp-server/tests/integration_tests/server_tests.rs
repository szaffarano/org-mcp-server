//! Integration tests for MCP server lifecycle and connection management.
//!
//! This module contains tests that verify basic server functionality including
//! startup, shutdown, connection establishment, and server information exchange.

use rmcp::transport::ConfigureCommandExt;
use tokio::process::Command;
use tracing::{error, info};
use tracing_test::traced_test;

use crate::{create_mcp_service, get_binary_path, setup_test_org_files};

/// Tests graceful server startup and shutdown.
///
/// Verifies that:
/// - The MCP server binary can be launched successfully
/// - Server process starts and runs without immediate termination
/// - Server can be gracefully terminated by closing stdin
/// - Process cleanup happens cleanly
#[traced_test]
#[tokio::test]
async fn test_graceful_close_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP server using pre-compiled binary");

    let org_dir = setup_test_org_files()?;
    let binary = get_binary_path("org-mcp-server");
    let mut command = Command::new(binary).configure(|cmd| {
        cmd.args(["--root-directory", org_dir.path().to_str().unwrap()]);
    });

    let mut child = command.stdin(std::process::Stdio::piped()).spawn()?;

    // Give the server a moment to start up
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Check that the server didn't exit prematurely
    match child.try_wait()? {
        Some(status) => {
            error!("MCP server exited prematurely with status: {}", status);
            return Err("MCP server exited prematurely".into());
        }
        None => {
            info!("MCP server is running as expected");
        }
    }

    // Gracefully terminate by closing stdin
    if let Some(stdin) = child.stdin.take() {
        drop(stdin);
    }

    // Wait for the server to terminate
    child.wait().await?;
    info!("MCP server process terminated gracefully");

    Ok(())
}

/// Tests basic MCP connection establishment and server information exchange.
///
/// Verifies that:
/// - Client can successfully connect to the MCP server
/// - Server responds with proper initialization and capability information
/// - Server info includes expected instructions and tool capabilities
/// - Connection can be cleanly terminated
#[tokio::test]
#[traced_test]
async fn test_mcp_server_connection() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP client to test server connection and handshake");

    let temp_dir = setup_test_org_files()?;
    let service = create_mcp_service!(&temp_dir);

    // Retrieve and verify server information
    let server_info = service.peer_info();
    info!("Connected to server: {:#?}", server_info);

    if let Some(info) = server_info {
        // Verify that server provides instructions for usage
        assert!(
            info.instructions.is_some(),
            "Server should provide usage instructions"
        );

        // Verify that server declares tool capabilities
        assert!(
            info.capabilities.tools.is_some(),
            "Server should declare tool capabilities"
        );

        // Verify that instructions are not empty
        let instructions = info.instructions.as_ref().unwrap();
        assert!(
            !instructions.is_empty(),
            "Server instructions should not be empty"
        );

        info!("Server capabilities verified successfully");
    } else {
        panic!("Expected server info to be present after connection");
    }

    // Clean up the connection
    service.cancel().await?;
    info!("MCP server connection test completed successfully");

    Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_mcp_server_with_config_file() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    info!("Starting MCP server with config file");

    let temp_dir = setup_test_org_files()?;
    let config_path = temp_dir.path().join("test-config.toml");

    // Convert path to forward slashes for TOML compatibility on Windows
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let config_content = format!(
        r#"
[org]
org_directory = "{}"

[logging]
level = "debug"
"#,
        path_str
    );
    fs::write(&config_path, config_content)?;

    use rmcp::{
        ServiceExt,
        transport::{ConfigureCommandExt, TokioChildProcess},
    };

    let command =
        tokio::process::Command::new(crate::get_binary_path("org-mcp-server")).configure(|cmd| {
            cmd.args(["--config", config_path.to_str().unwrap()]);
        });

    let service = ().serve(TokioChildProcess::new(command)?).await?;

    let server_info = service.peer_info();
    assert!(
        server_info.is_some(),
        "Server should be connected with config file"
    );

    service.cancel().await?;
    info!("MCP server with config file test completed successfully");

    Ok(())
}

#[tokio::test]
async fn test_tools_list_has_cache_hints_for_legacy_protocol()
-> Result<(), Box<dyn std::error::Error>> {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let org_dir = setup_test_org_files()?;
    let mut child = Command::new(get_binary_path("org-mcp-server"))
        .args(["--root-directory", org_dir.path().to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();

    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    ];
    for message in messages {
        stdin.write_all(format!("{message}\n").as_bytes()).await?;
    }

    let mut result = None;
    while let Some(line) = stdout.next_line().await? {
        let value: serde_json::Value = serde_json::from_str(&line)?;
        if value["id"] == 2 {
            result = Some(value["result"].clone());
            break;
        }
    }
    let result = result.ok_or("no tools/list response")?;

    assert_eq!(result["ttlMs"], 0);
    assert_eq!(result["cacheScope"], "public");
    assert_eq!(result["tools"].as_array().map(Vec::len), Some(5));
    Ok(())
}
