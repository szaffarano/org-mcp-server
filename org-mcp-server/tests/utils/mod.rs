//! Utility functions and macros for MCP server integration tests.
//!
//! This module provides reusable components for testing the org-mcp-server,
//! including test data setup, server connection helpers, and path utilities.

use std::{
    env::{self, consts},
    path::{self, PathBuf},
};

use tempfile::TempDir;

/// Gets the path to a compiled binary in the target directory.
///
/// This function first checks for a Cargo-provided environment variable
/// (used during `cargo test`), then falls back to constructing the path
/// from the target directory.
///
/// # Arguments
/// * `name` - The name of the binary to locate
///
/// # Returns
/// * `PathBuf` - The path to the binary executable
///
/// # Example
/// ```rust
/// let server_path = get_binary_path("org-mcp-server");
/// ```
pub fn get_binary_path(name: &str) -> PathBuf {
    env::var_os(format!("CARGO_BIN_EXE_{name}"))
        .map(|p| p.into())
        .unwrap_or_else(|| target_dir().join(format!("{}{}", name, consts::EXE_SUFFIX)))
}

/// Determines the target directory for compiled binaries.
///
/// This function walks up from the current executable's location to find
/// the target directory where Cargo places compiled binaries.
///
/// # Returns
/// * `PathBuf` - The path to the target directory
///
/// # Panics
/// Panics if the current executable path cannot be determined.
pub fn target_dir() -> path::PathBuf {
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

/// Creates a temporary directory with comprehensive test org-mode files.
///
/// This function sets up a complete test environment with multiple org files
/// containing various org-mode features like headings, properties, TODO items,
/// and nested directory structures using shared fixtures from test-utils.
///
/// # Test File Structure
/// The created directory contains:
///
/// * `notes.org` - Main notes file with:
///   - Daily Tasks (ID: daily-tasks-123)
///   - TODO and DONE items (IDs: task-groceries-456, task-book-789)
///   - Project Ideas and Meeting Notes
///
/// * `project.org` - Project planning file with:
///   - Backend Development (ID: backend-dev-101)
///   - API Design (ID: api-design-102)
///   - Frontend Development (ID: frontend-dev-201)
///
/// * `research.org` - Research notes with:
///   - Machine Learning section (ID: ml-research-301)
///   - Deep Learning subsection (ID: dl-fundamentals-302)
///   - Rust Programming (ID: rust-programming-401)
///
/// * `archive/old_notes.org` - Archived content with:
///   - Archived Ideas (ID: archived-ideas-501)
///   - Completed Projects and Learning Resources
///
/// # Returns
/// * `Result<TempDir, Box<dyn std::error::Error>>` - Temporary directory with test files
///
/// # Example
/// ```rust
/// let temp_dir = setup_test_org_files()?;
/// let service = create_mcp_service!(&temp_dir);
/// // Use service to test org file operations
/// ```
///
/// # Test IDs Reference
/// For testing ID-based lookups, these IDs are available:
/// - `daily-tasks-123`: Main Daily Tasks heading
/// - `task-groceries-456`: TODO Buy groceries item
/// - `task-book-789`: DONE Read book item
/// - `backend-dev-101`: Backend Development section
/// - `api-design-102`: API Design subsection
/// - `frontend-dev-201`: Frontend Development section
/// - `ml-research-301`: Machine Learning section
/// - `dl-fundamentals-302`: Deep Learning Fundamentals
/// - `rust-programming-401`: Rust Programming section
/// - `archived-ideas-501`: Archived Ideas section
pub fn setup_test_org_files() -> Result<TempDir, Box<dyn std::error::Error>> {
    test_utils::fixtures::setup_test_org_files()
}
