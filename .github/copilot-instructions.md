# org-mcp-server Development Guide

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Project Overview

Multi-crate Rust workspace for org-mode/roam knowledge base management via Model Context Protocol (MCP). Three main crates:

- `org-core` — Business logic and org-mode functionality  
- `mcp-server` — MCP protocol implementation
- `org-cli` — CLI tool for testing and direct usage

## Working Effectively

### Prerequisites
- Rust toolchain (2024 edition)
- Directory `~/org/` must exist for MCP server to start

### Bootstrap and Build
- `cargo build` — Build all crates. Takes ~22 seconds. NEVER CANCEL. Set timeout to 90+ minutes.
- `cargo build --release` — Release build. Takes ~49 seconds. NEVER CANCEL. Set timeout to 90+ minutes.

### Testing  
- `cargo test` — Run all tests (36 tests). Takes ~2 seconds. NEVER CANCEL. Set timeout to 15+ minutes.
- `cargo test -p org-core` — Test core crate specifically (~27 tests). Takes ~9 seconds. 
- `cargo test -p mcp-server` — Test server crate (~9 tests).
- `cargo test <test_name>` — Run specific test by name.

### Code Quality
- `cargo fmt` — Format code. Takes <1 second.
- `cargo fmt --check` — Check formatting without changes. Takes <1 second.
- `cargo clippy` — Run linter. Takes ~19 seconds. NEVER CANCEL. Set timeout to 30+ minutes.

### Running Applications

#### CLI Tool
```bash
# Basic commands
cargo run --bin org-cli -- --help
cargo run --bin org-cli -- list --dir <directory>
cargo run --bin org-cli -- outline --dir <directory> <file.org>
cargo run --bin org-cli -- element-by-id <id>
```

#### MCP Server
```bash
# Start MCP server (requires ~/org/ directory to exist)
mkdir -p ~/org  # Required first step
cargo run --bin mcp-server

# With debug logging
RUST_LOG=debug cargo run --bin mcp-server
```

#### Examples
```bash
cargo run --example orgize_parser-01 "* Hello World"
```

## Validation Scenarios

### Always Test These After Changes

1. **Build validation**: Run `cargo build` and `cargo build --release` to ensure compilation succeeds.

2. **Test validation**: Run `cargo test` to ensure all 36 tests pass.

3. **CLI functionality**: 
   ```bash
   mkdir -p ~/org
   echo "* Test\n** Subtask\n:PROPERTIES:\n:ID: test-123\n:END:" > ~/org/test.org
   cargo run --bin org-cli -- list
   cargo run --bin org-cli -- outline test.org  
   cargo run --bin org-cli -- element-by-id test-123
   ```

4. **MCP server startup**:
   ```bash
   mkdir -p ~/org
   timeout 5s cargo run --bin mcp-server  # Should start without errors
   ```

5. **Code quality**: Run `cargo fmt --check` and `cargo clippy` to ensure style compliance.

## Build Timing Expectations

- `cargo build`: ~22 seconds — NEVER CANCEL. Use 90+ minute timeout.
- `cargo build --release`: ~49 seconds — NEVER CANCEL. Use 90+ minute timeout.  
- `cargo test`: ~2 seconds — NEVER CANCEL. Use 15+ minute timeout.
- `cargo test -p org-core`: ~9 seconds — NEVER CANCEL. Use 15+ minute timeout.
- `cargo clippy`: ~19 seconds — NEVER CANCEL. Use 30+ minute timeout.
- `cargo fmt`: <1 second

## Repository Structure

```
org-mcp-server/
├── Cargo.toml           # Workspace definition
├── Cargo.lock           # Dependency lockfile
├── README.org           # Project documentation
├── CLAUDE.md            # Development guidelines
├── flake.nix            # Nix development environment
├── .envrc               # direnv configuration
├── org-core/            # Core business logic crate
│   ├── examples/        # Playground examples
│   ├── tests/fixtures/  # Test org files
│   └── src/
├── mcp-server/          # MCP protocol server crate
│   └── src/
└── org-cli/             # CLI interface crate
    └── src/
```

## Common Tasks

### Adding New Features
1. Implement core logic in `org-core`
2. Expose via MCP resources/tools in `mcp-server`  
3. Add CLI command in `org-cli`
4. Add tests with fixtures in `org-core/tests/`
5. Run full validation suite

### Testing With Fixtures
Test fixtures available in `org-core/tests/fixtures/`:
- `simple.org` — Basic org file
- `nested.org` — Multi-level headings
- `with_ids.org` — Files with ID properties
- `multi_file_a.org`, `multi_file_b.org` — Multi-file scenarios

Example usage:
```bash
cargo run --bin org-cli -- list --dir org-core/tests/fixtures
cargo run --bin org-cli -- outline --dir org-core/tests/fixtures nested.org
```

### Environment Configuration
- `RUST_LOG=debug` — Enable debug logging for MCP server
- `RUST_BACKTRACE=1` — Enable backtraces for debugging

## Nix Development Environment

This project uses Nix flakes for development environment management:
- `nix develop` — Enter development shell (if Nix available)
- `nix run .` — Run the application via Nix
- Pre-commit hooks configured for formatting and linting

**Note**: Nix is optional. Standard Rust toolchain works fine.

## Dependencies and Architecture

Key dependencies:
- `orgize` — Org-mode parsing
- `rmcp` — MCP protocol implementation  
- `tokio` — Async runtime
- `clap` — CLI argument parsing
- `walkdir` — File system traversal

## Error Handling Notes

- MCP server requires `~/org/` directory to exist
- CLI tools accept `--dir` parameter for custom directories
- ID-based lookups are case-sensitive
- File paths should be relative to the configured org directory

## Development Workflow Tips

1. **Format first**: Always run `cargo fmt` before committing
2. **Test incrementally**: Run `cargo test` after each logical change
3. **Use fixtures**: Test with files in `org-core/tests/fixtures/` 
4. **Check examples**: Run examples to validate org-mode parsing works
5. **Validate CLI**: Test both list and outline operations before finalizing
6. **Verify MCP startup**: Ensure server starts cleanly after changes

## Known Issues

- MCP server does not accept command line arguments for org directory (TODO in main.rs)
- Logging configuration hardcoded (TODO in main.rs)
- Default directory is `~/org/` which must exist

## Quick Reference Commands

```bash
# Full validation sequence
cargo fmt --check && cargo clippy && cargo build && cargo test

# Test with sample data
mkdir -p ~/org && echo "* Test" > ~/org/test.org
cargo run --bin org-cli -- list
cargo run --bin org-cli -- outline test.org

# Quick development cycle
cargo test -p org-core && cargo run --bin org-cli -- --help
```