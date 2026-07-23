# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GitHub repo: <https://github.com/szaffarano/org-mcp-server/>

MCP server for org-mode knowledge management in Rust. Provides search, content access, agenda views, and note linking for org-mode files through the [Model Context Protocol](https://modelcontextprotocol.io/).

## Development Commands

```bash
cargo build                          # Build all crates (~47s, never cancel)
cargo test                           # Run all tests
cargo test <test_name>               # Run specific test
cargo test -p <crate_name>           # Test specific crate (org-core, org-mcp-server, org-cli)
cargo test --test integration_tests  # Run MCP server integration tests
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo fmt --all                      # Format code
cargo run --bin org-cli -- --help    # Run CLI tool
cargo run --bin org-mcp-server       # Run MCP server (needs valid org directory)
cargo run --example <name>           # Run playground examples (in org-core/examples/)
```

### Just Commands (available in nix devShell)

```bash
just dev          # Full workflow: format, lint, test, coverage
just check        # Quality gate: fmt-check, lint, test, coverage
just test         # Run all tests
just lint         # Clippy with -D warnings
just fmt          # Format code
just coverage-html # HTML coverage in coverage/html/
```

## Architecture

Rust 2024 edition, multi-crate workspace with async-first design (`tokio`).

### Crate Dependency Flow

```text
org-core  <--  org-mcp-server
org-core  <--  org-cli
test-utils <-- org-core (dev), org-mcp-server (dev), org-cli (dev)
```

- **`org-core`** — All business logic: org-mode parsing (`orgize`), file discovery (`ignore` crate's `Walk`), fuzzy search (`nucleo-matcher`), agenda/task querying, configuration loading (`config` crate with TOML/YAML/JSON + env vars). Central type is `OrgMode` struct (`org-core/src/org_mode/types.rs`) which holds an `OrgConfig` and provides all operations. Custom error type `OrgModeError` in `org-core/src/error.rs`.

- **`org-mcp-server`** — MCP protocol layer using `rmcp`. `OrgModeRouter` (`src/core.rs`) wraps `Arc<Mutex<OrgMode>>` and a `ToolRouter`. Resources (`src/resources/mod.rs`) implement `ServerHandler` with URI-based routing (org://, org-outline://, org-heading://, org-id://, org-agenda://). Tools (`src/tools/`) expose org-file-list, org-search, org-agenda, org-capture. Server runs over stdio transport.

- **`org-cli`** — CLI using `clap` with subcommands: config, list, read, outline, heading, element-by-id, search, agenda, capture. Each command in `src/commands/`. Delegates to `OrgMode` from org-core. The `Commands::Capture` enum variant is boxed (`Box<CaptureCommand>`) to keep the enum size small.

- **`test-utils`** — Shared test infrastructure. Fixtures in `test-utils/fixtures/` (20 .org files). Key helpers: `setup_test_org_files()` copies fixtures to temp dir, `setup_test_org_files_with_dates()` replaces date placeholders (`@TODAY@`, `@TODAY+N@`) for time-sensitive agenda tests.

### Key Dependencies

- `orgize` (custom fork) — org-mode parsing, traversal via `from_fn`/`from_fn_with_ctx` handlers
- `rmcp` — MCP protocol (tools, resources, stdio transport)
- `nucleo-matcher` — fuzzy text search
- `ignore` — gitignore-aware file walking
- `config` — layered configuration (file + env + defaults)

### Configuration

TOML config at `~/.config/org-mcp/config.toml`. Layered: defaults < config file < env vars (`ORG_` prefix, `__` separator) < CLI flags. See `org-core/src/config.rs` for `OrgConfig` and `LoggingConfig`. Notable capture-related field: `org_auto_created_property: bool` (default `true`) controls whether `capture_append` prepends a `:CREATED:` property.

### Capture path internals

`OrgMode::capture_append` (in `org-core/src/org_mode/capture.rs`) is the central write path. Highlights:

- Per-target `<file>.lock` sibling file for cross-process serialization, with stat-after-lock verification so the lockfile can be safely unlinked on release.
- Atomic write via `tempfile.persist`-style temp file + `fsync` + `rename` so a crash mid-write cannot truncate user data.
- Validation order: title → level → todo_state → priority → tags → planning timestamps → datetree → properties → auto-CREATED prepend.
- `parse_iso_timestamp` accepts `YYYY-MM-DD [HH:MM] [repeater] [warning]`. `format_org_timestamp` renders the canonical `<...>` / `[...]` form.
- Datetree expansion prepends Year / Year-Month-Name / Year-Month-Day-Day segments to the resolved heading path before `find_heading_path` runs; the existing missing-segment creation logic handles first-of-day creation and idempotent reuse.

## Code Style

- `cargo fmt` before every commit
- String interpolation: `"string {var}"` not `"string {}", var`
- Imports: std before external crates
- Tests: `assert_eq!` over `assert!`, `#[cfg(test)]` modules for unit tests
- Error handling: `Result<T, OrgModeError>`, never panic
- Workspace deps defined in root `Cargo.toml`, crates reference with `workspace = true`

## Development Workflow

1. New functionality goes into `org-core`, then gets exposed via `org-mcp-server` and/or `org-cli`
2. Multi-crate dep changes go in root `Cargo.toml` `[workspace.dependencies]`
3. Test fixtures for new org-mode features go in `test-utils/fixtures/`
4. Integration tests for MCP server use the `create_mcp_service!` macro that spawns the server binary as a child process
5. Config tests using env vars must use `#[serial]` from `serial_test` crate
6. Always run `cargo clippy` and `cargo fmt` before committing

## Behavioral Guidelines

- Concise responses, be direct
- NEVER create files unless absolutely necessary
- Commits: always sign with `-S`
- Documentation: only create when explicitly requested
- Use the Context7 mcp server to get docs and code examples
