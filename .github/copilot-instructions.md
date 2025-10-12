# org-mcp-server

org-mcp-server is a Rust Model Context Protocol (MCP) server for org-mode
knowledge management. It provides search, content access, and note linking
capabilities for org-mode files through the MCP protocol.

**ALWAYS** reference these instructions first and fallback to the CLAUDE.md
file, search, or bash commands only when you encounter unexpected information
that does not match the info here.

## Working Effectively

### Bootstrap and Build

- **NEVER CANCEL builds or tests** - they may take up to 90 seconds to complete
- Build the workspace: `cargo build` -- takes ~47 seconds. **NEVER CANCEL. Set
  timeout to 90+ seconds.**
- Release build: `cargo build --release` -- takes ~47 seconds. **NEVER CANCEL.
  Set timeout to 90+ seconds.**
- Run tests: `cargo test` -- takes ~2 seconds, 36 tests. Safe timeout 30+
  seconds.
- Lint code: `cargo clippy` -- takes ~11 seconds. Safe timeout 30+ seconds.
- Format code: `cargo fmt` -- takes \<1 second. Always run before commits.
- Check formatting: `cargo fmt --check` -- takes \<1 second.

### Development Commands

- Run CLI tool: `cargo run --bin org-cli -- --help`
- Run MCP server: `cargo run --bin org-mcp-server` (needs valid org directory)
- Run examples: `cargo run --example <name>` (examples in `org-core/examples/`)
- Test specific crate: `cargo test -p <crate-name>`
- Build specific crate: `cargo build -p <crate-name>`

## Validation

### CRITICAL: Always Test These Scenarios After Changes

1. **Build validation**: Run `cargo build` and wait for completion (90+ seconds)

1. **Test validation**: Run `cargo test` and verify all 36 tests pass

1. **CLI validation**: Test core CLI functionality:

   ```bash
   cargo run --bin org-cli -- list --dir org-core/tests/fixtures
   cargo run --bin org-cli -- read --dir org-core/tests/fixtures sample.org
   cargo run --bin org-cli -- outline --dir org-core/tests/fixtures sample.org
   cargo run --bin org-cli -- element-by-id --dir org-core/tests/fixtures simple-123
   ```

1. **Linting validation**: Run `cargo clippy` and `cargo fmt --check`

### Manual Testing Requirements

- **ALWAYS** test the CLI with actual org files after making changes
- **ALWAYS** verify that the core functionality works with test fixtures
- Use `org-core/tests/fixtures/` for testing - contains 8 org files with
  various structures
- Test files include: `sample.org`, `with_ids.org`, `nested.org`, etc.

## Project Structure

### Multi-Crate Workspace

- **org-core** — Business logic and org-mode parsing (`org-core/src/`)
- **mcp-server** — MCP protocol implementation (`mcp-server/src/`)
- **org-cli** — CLI interface for testing (`org-cli/src/`)

### Key Directories

- `org-core/examples/` — Playground examples for experimentation
- `org-core/tests/fixtures/` — Test org files for validation
- `org-core/src/` — Core business logic (lib.rs, org_mode.rs, error.rs)
- `mcp-server/src/` — MCP server (main.rs, core.rs, resources/, tools/)
- `org-cli/src/` — CLI tool implementation

### Configuration Files

- `Cargo.toml` — Workspace configuration
- `flake.nix` — Nix development environment
- `CLAUDE.md` — Development guidelines
- `README.md` — Project documentation

## Architecture and Dependencies

### Tech Stack

- **Rust 2024 edition** with async-first design using `tokio`
- **orgize** for org-mode parsing
- **rmcp** for MCP protocol implementation
- **clap** for CLI interface
- **walkdir** for file traversal
- **serde/serde_json** for serialization

### Error Handling

- Use custom error types with proper chaining
- Prefer explicit `Result<T, E>` over panics
- Check `org-core/src/error.rs` for error definitions

## Development Workflow

### Making Changes

1. **Always** start by running the full validation suite
1. Make focused, minimal changes
1. **Build immediately** after changes: `cargo build` (wait 90+ seconds)
1. **Test immediately** after changes: `cargo test`
1. **Always** run CLI validation scenarios
1. **Always** run `cargo clippy` and `cargo fmt` before committing

### Code Style

- Use `cargo fmt` before all commits
- Use `"string {var}"` over `"string {}", var`
- Standard library imports before external crates
- Add `#[cfg(test)]` modules for tests
- Use `assert_eq!` over `assert!` in tests

### Testing Strategy

- Unit tests in each crate with `#[cfg(test)]` modules
- Integration tests use fixtures in `org-core/tests/fixtures/`
- **CRITICAL**: Always test with real org files using CLI commands
- Test files contain various org-mode features (headings, IDs, properties)

## Common Tasks

### Repository Root Structure

```sh
.
├── Cargo.toml          # Workspace config
├── README.org          # Project documentation
├── CLAUDE.md           # Development guidelines
├── flake.nix           # Nix development environment
├── org-core/           # Core business logic
│   ├── src/
│   ├── examples/       # Experimentation examples
│   └── tests/fixtures/ # Test org files
├── mcp-server/         # MCP protocol server
│   └── src/
└── org-cli/            # CLI tool
    └── src/
```

### Frequently Used Commands Output

**CLI Help**:

```sh
$ cargo run --bin org-cli -- --help
A CLI tool for org-mode functionality

Usage: org-cli <COMMAND>

Commands:
  list           List all .org files in a directory
  init           Initialize or validate an org directory
  read           Read the contents of an org file
  outline        Get the outline (headings) of an org file
  heading        Extract content from a specific heading
  element-by-id  Extract content from an element by ID
  help           Print this message or help of subcommand
```

**Test Fixtures List**:

```sh
$ cargo run --bin org-cli -- list --dir org-core/tests/fixtures
Found 8 .org files in org-core/tests/fixtures:
  edge_cases.org
  doc_with_id.org
  nested.org
  sample.org
  multi_file_a.org
  with_ids.org
  multi_file_b.org
  simple.org
```

## Key Implementation Details

### ID-Based Element Lookup

- Use `element-by-id` command to find org elements by ID property
- Test with: `cargo run --bin org-cli -- element-by-id --dir org-core/tests/fixtures simple-123`
- IDs are defined in `:PROPERTIES:` blocks in org files

### File Operations

- All file paths are validated at construction, not runtime
- Use `shellexpand` for path expansion
- Default org directory is `~/org/` but can be overridden

### MCP Server Limitations

- Current MCP server implementation needs directory configuration improvements
- Server provides resources: `org://`, `org://file`, `org-outline://`, \`org-he
- Tools available: `org-file-list`

## Nix Development Environment

- Use `nix develop` for development shell with all dependencies
- Flake provides rust toolchain, cargo tools, and formatters
- Pre-commit hooks available for formatting and linting

## CRITICAL REMINDERS

- **NEVER CANCEL** build commands - they may take 90+ seconds
- **ALWAYS** validate with CLI scenarios after changes
- **ALWAYS** run the complete test suite before committing
- **ALWAYS** use appropriate timeouts (90+ seconds for builds, 30+ seconds for tests)
- Build times are normal - do not attempt to interrupt or optimize prematurely
