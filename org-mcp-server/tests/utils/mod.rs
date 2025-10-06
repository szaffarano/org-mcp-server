//! Utility functions and macros for MCP server integration tests.
//!
//! This module provides reusable components for testing the org-mcp-server,
//! including test data setup, server connection helpers, and path utilities.

use std::{
    env::{self, consts},
    fs,
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
/// and nested directory structures.
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
    let temp_dir = TempDir::new()?;

    // Create main notes file with various org-mode features
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

    // Create project planning file with nested structure
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

    // Create research notes with technical content
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

    // Create archive subdirectory with old files
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
