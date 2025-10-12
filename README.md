# org-mcp-server

[![CI](https://github.com/szaffarano/org-mcp-server/actions/workflows/ci.yml/badge.svg)](https://github.com/szaffarano/org-mcp-server/actions/workflows/ci.yml)
[![Coverage](https://github.com/szaffarano/org-mcp-server/actions/workflows/coverage.yml/badge.svg)](https://github.com/szaffarano/org-mcp-server/actions/workflows/coverage.yml)
[![codecov](https://codecov.io/gh/szaffarano/org-mcp-server/branch/master/graph/badge.svg)](https://codecov.io/gh/szaffarano/org-mcp-server)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/szaffarano/org-mcp-server/blob/master/LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%2B-orange.svg)](https://github.com/szaffarano/org-mcp-server)
[![Dependency Status](https://deps.rs/repo/github/szaffarano/org-mcp-server/status.svg)](https://deps.rs/repo/github/szaffarano/org-mcp-server)

**🚧 Work in Progress**: This project is under active development.

A [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server for
org-mode knowledge management. Provides search, content access, and note
linking capabilities for your org-mode files through the MCP protocol.

## Features

### MCP Resources

- `org://` — List all org-mode files in configured directories
- `org://{file}` — Access raw content of `{file}`
- `org-outline://{file}` — Get hierarchical structure of `{file}` as JSON
- `org-heading://{file}#{heading}` — Access specific headings by path
- `org-id://{id}` — Find content by org-mode ID properties

### MCP Tools

- `org-file-list` — List all org files in configured directories
- `org-search` — Search for text content across all org files using fuzzy matching

### CLI Tool

- `org-cli config init` — Create default configuration file
- `org-cli config show` — Display current configuration
- `org-cli config path` — Show configuration file location
- `org-cli list` — List all .org files in configured directory
- `org-cli init` — Initialize or validate an org directory
- `org-cli read` — Read the contents of an org file
- `org-cli outline` — Get the outline (headings) of an org file
- `org-cli heading` — Extract content from a specific heading in an org file
- `org-cli element-by-id` — Extract content from an element by ID across all
  org files
- `org-cli search` — Search for text content across all org files using fuzzy
  matching

## Configuration

The project uses a TOML configuration file located at
`~/.config/org-mcp-server.toml` (or `$XDG_CONFIG_HOME/org-mcp-server.toml`).

### Configuration Hierarchy

Configuration is resolved in the following order (highest priority first):

1. **CLI flags** — Command-line arguments override everything
2. **Environment variables** — `ORG_*` prefixed variables
3. **Configuration file** — TOML file in config directory
4. **Default values** — Built-in fallbacks

### Configuration File Format

```toml
[org]
# Root directory containing org-mode files
root_directory = "~/org/"
# Default notes file for new notes
default_notes_file = "notes.org"
# Agenda files to include
agenda_files = ["agenda.org", "projects.org"]
# Extra files for text search beyond regular org files
agenda_text_search_extra_files = ["archive.org"]

[logging]
# Log level: trace, debug, info, warn, error
level = "info"
# Log file location (MCP server only, CLI logs to stderr)
file = "~/.local/share/org-mcp-server/logs/server.log"

[cli]
# Default output format for CLI commands
default_format = "plain"  # plain | json
```

### Environment Variables

- `ORG_ROOT_DIRECTORY` — Root directory for org files
- `ORG_DEFAULT_NOTES_FILE` — Default notes file name
- `ORG_AGENDA_FILES` — Comma-separated list of agenda files
- `ORG_AGENDA_TEXT_SEARCH_EXTRA_FILES` — Comma-separated extra search files
- `ORG_LOG_LEVEL` — Log level for server
- `ORG_LOG_FILE` — Log file location for server

### Configuration Commands

```bash
# Create default configuration file
org config init

# Show current resolved configuration
org config show

# Show configuration file path
org config path
```

## Usage Examples

### Basic Commands

```bash
# List all org files using configuration
org list

# List with JSON output
org list --format json

# Search across all configured org files
org search "project planning"

# Search with custom parameters
org search "TODO" --limit 5 --format json --snippet-size 75

# Override root directory for a single command
org --root-directory ~/documents/org search "meeting notes"
```

## Architecture

Multi-crate Rust workspace:

- **org-core** — Business logic and org-mode parsing
- **org-mcp-server** — MCP protocol implementation
- **org-cli** — CLI interface for testing and direct usage

Built with:

- [orgize](https://crates.io/crates/orgize) for org-mode parsing
- [rmcp](https://crates.io/crates/rmcp) for MCP protocol
- [tokio](https://crates.io/crates/tokio) for async runtime
- [nucleo-matcher](https://crates.io/crates/nucleo-matcher) for fuzzy text search

## Setup

### Using Nix Flakes

```bash
# Run directly with nix
nix run github:szaffarano/org-mcp-server

# Install to profile
nix profile install github:szaffarano/org-mcp-server

# Development environment
nix develop github:szaffarano/org-mcp-server
```

### From Source

```bash
# Clone and build
git clone https://github.com/szaffarano/org-mcp-server
cd org-mcp-server
cargo build --release

# Run MCP server
cargo run --bin org-mcp-server

# Test with CLI
cargo run --bin org-cli -- list
```

## MCP Server Integration

### AI Agent Configuration

Add the following to your agent configuration (e.g.,
`~/.config/opencode/opencode.json`, `~/.claude.json`, etc.):

```json
{
  "mcpServers": {
    "org-mode": {
      "command": "/path/to/org-mcp-server",
      "args": [],
      "env": {}
    }
  }
}
```

Or if installed via Nix:

```json
{
  "mcpServers": {
    "org-mode": {
      "command": "nix",
      "args": ["run", "github:szaffarano/org-mcp-server"],
      "env": {}
    }
  }
}
```

### Environment Variable Configuration

You can configure the MCP server through environment variables in your agent configuration:

```json
{
  "mcpServers": {
    "org-mode": {
      "command": "/path/to/org-mcp-server",
      "args": [],
      "env": {
        "ORG_ROOT_DIRECTORY": "/path/to/your/org/files",
        "ORG_LOG_LEVEL": "info"
      }
    }
  }
}
```

## Development

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p org-core

# Format and lint
cargo fmt
cargo clippy

# Run examples
cargo run --example <name>
```

## Roadmap

### Phase 1: Core Functionality ✅

- [x] File discovery and listing
- [x] Basic content access via MCP resources
- [x] Org-mode parsing with orgize
- [x] ID-based element lookup
- [x] CLI tool for testing
- [x] Full-text search across org files

### Phase 2: Advanced Features 🚧

- [x] Configuration file support with TOML format
- [x] Environment variable configuration
- [x] Unified CLI interface with global configuration
- [x] Tag-based filtering and querying
- [ ] Link following and backlink discovery (org-roam support)
- [ ] Metadata caching for performance
- [ ] Agenda-related Functionality

### Phase 3: Extended Capabilities 📋

- [ ] Content creation and modification tools
- [ ] Media file reference handling
- [ ] Integration with org-roam databases
- [ ] Real-time file watching and updates
- [ ] Advanced query language

## License

[MIT License](LICENSE) - see LICENSE file for details.
