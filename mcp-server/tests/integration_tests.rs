//! Integration test suite for the org-mcp-server.
//!
//! This module organizes integration tests into logical groups and provides
//! shared utilities for testing MCP server functionality.
//!
//! ## Test Organization
//!
//! * `server_tests` - Server lifecycle and connection tests
//! * `tools_tests` - Tool functionality tests (org-file-list, org-search)
//! * `resources_tests` - Resource access tests (org://, org-outline://, etc.)
//! * `utils` - Shared test utilities and helper functions
//!
//! ## Running Tests
//!
//! Run all integration tests:
//! ```bash
//! cargo test --test integration_tests
//! ```
//!
//! Run specific test categories:
//! ```bash
//! cargo test --test integration_tests server_tests
//! cargo test --test integration_tests tools_tests
//! cargo test --test integration_tests resources_tests
//! ```

pub(crate) mod utils;
pub(crate) mod integration_tests {
    pub(crate) mod resources_tests;
    pub(crate) mod server_tests;
    pub(crate) mod tools_tests;
}

// Re-export the macro and utilities for use by test modules
pub(crate) use utils::{get_binary_path, setup_test_org_files};

#[macro_export]
macro_rules! create_mcp_service {
    ($temp_dir:expr) => {{
        use rmcp::{
            ServiceExt,
            transport::{ConfigureCommandExt, TokioChildProcess},
        };
        use tracing::error;

        let command = tokio::process::Command::new($crate::get_binary_path("org-mcp-server"))
            .configure(|cmd| {
                cmd.args(["--root", $temp_dir.path().to_str().unwrap()]);
            });

        ().serve(TokioChildProcess::new(command)?)
            .await
            .map_err(|e| {
                error!("Failed to connect to server: {}", e);
                e
            })?
    }};
}
