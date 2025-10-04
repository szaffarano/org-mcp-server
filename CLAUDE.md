# Claude Guidelines for org-mcp-server

## Project Context

MCP server for org-mode/roam knowledge base management in Rust. Multi-crate
workspace with:

- `org-core` — Business logic and org-mode functionality
- `mcp-server` — MCP protocol implementation
- `org-cli` — CLI tool for testing and direct usage

**Goal**: Provide search, content creation, and note linking with media
references for org-mode files.

## Development Commands

- `cargo build` — Build all crates
- `cargo test` — Run all tests
- `cargo test <test_name>` — Run specific test
- `cargo test -p <crate_name>` — Test specific crate
- `cargo clippy` — Run linter
- `cargo fmt` — Format code
- `cargo run --example <name>` — Run playground examples
- `cargo run --bin org-cli` — Run CLI tool
- `cargo run --bin org-mcp-server` — Run MCP server

## Code Style & Preferences

- **Formatting**: Always use `cargo fmt` before commits
- **Error handling**: Prefer explicit `Result<T, E>` over panics
- **String formatting**: Use `"string {var}"` over `"string {}", var`
- **Imports**: Standard library before external crates
- **Testing**: Use `assert_eq!` over `assert!`, add `#[cfg(test)]` modules
- **Functions**: Keep focused and well-documented

## Architecture

- **Rust 2024 edition** with async-first design using `tokio`
- **Examples** in `org-core/examples/` for dependency experimentation
- **Test fixtures** in `tests/fixtures/` for org/roam files
- **Key deps**: `orgize` (parsing), `walkdir` (file traversal), `clap` (CLI)

## Development Workflow

1. **Multi-crate changes**: Update workspace dependencies in root Cargo.toml
1. **New functionality**: Add to `org-core`, expose via `mcp-server` and `org-cli`
1. **Error handling**: Use custom error types, implement proper chaining
1. **File operations**: Validate paths at construction, not runtime
1. **Testing**: Create fixtures for complex org-mode files

## Behavioral Guidelines

- **Concise responses**: Be direct, avoid unnecessary explanations
- **File creation**: NEVER create files unless absolutely necessary
- **Commits**: Always sign with -S, never include Claude Code references
- **Code quality**: Run clippy and fmt before suggesting changes
- **Documentation**: Only create when explicitly requested

## Current Implementation Status

- ✅ Basic file listing with recursive directory traversal
- ✅ Error handling with custom types and proper chaining
- ✅ CLI tool with `list` and `init` commands
- ✅ MCP server with JSON-RPC protocol
- 🚧 Org-mode parsing and content extraction (planned)
- 🚧 Search functionality with metadata caching (planned)
