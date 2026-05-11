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
- `org-agenda://` — List all agenda items and tasks
- `org-agenda://today` — Today's scheduled agenda items
- `org-agenda://week` — This week's scheduled agenda items

### MCP Tools

- `org-file-list` — List all org files in configured directories
- `org-search` — Search for text content across all org files using fuzzy matching
- `org-agenda` — Query agenda items with filtering by dates, states, tags, and priorities
- `org-capture` — Append a fully-formed heading to an org file. Supports TODO state,
  priority, tags, body, SCHEDULED/DEADLINE/CLOSED timestamps (with optional repeater
  and warning suffixes), property drawer entries, and Year/Month/Day datetree expansion.
  Can target a specific heading or append to end of file.

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
- `org-cli agenda list` — List all tasks (TODO/DONE items)
- `org-cli agenda today` — Show today's scheduled tasks
- `org-cli agenda week` — Show this week's scheduled tasks
- `org-cli agenda range` — Show tasks in custom date range
- `org-cli capture` — Append a new heading to an org file with optional TODO state,
  priority, tags, body, planning timestamps, property drawer, and datetree expansion

## Configuration

The project uses a TOML configuration file located at
`~/.config/org-mcp/config.toml` (or `$XDG_CONFIG_HOME/org-mcp/config.toml`).

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
org_directory = "~/org/"
# Default notes file for new notes
org_default_notes_file = "notes.org"
# Agenda files to include
org_agenda_files = ["agenda.org", "projects.org"]
# Extra files for text search beyond regular org files
org_agenda_text_search_extra_files = ["archive.org"]
org_todo_keywords = [
    "TODO",
    "|",
    "DONE",
]
# When true, capture automatically prepends an inactive timestamp like
# `:CREATED: [YYYY-MM-DD Day HH:MM]` (with the current local time) to the property
# drawer of new entries. User-supplied CREATED (case-insensitive) wins.
org_auto_created_property = true

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

#### Org-mode Configuration
- `ORG_ORG__ORG_DIRECTORY` — Root directory for org files
- `ORG_ORG__ORG_DEFAULT_NOTES_FILE` — Default notes file name
- `ORG_ORG__ORG_AGENDA_FILES` — Comma-separated list of agenda files
- `ORG_ORG__ORG_AGENDA_TEXT_SEARCH_EXTRA_FILES` — Comma-separated extra search files
- `ORG_ORG__ORG_AUTO_CREATED_PROPERTY` — `true`/`false`; auto-add `:CREATED:` on capture (default: true)

#### Logging Configuration
- `ORG_LOGGING__LEVEL` — Log level (debug, info, warn, error, trace)
- `ORG_LOGGING__FILE` — Log file location

#### Server Configuration
- `ORG_SERVER__MAX_CONNECTIONS` — Maximum number of concurrent connections (default: 10)

#### CLI Configuration
- `ORG_CLI__DEFAULT_FORMAT` — Default output format for CLI commands (plain, json)

### Configuration Commands

```bash
# Create default configuration file
org-cli config init

# Show current resolved configuration
org-cli config show

# Show configuration file path
org-cli config path
```

## Usage Examples

### Basic Commands

```bash
# List all org files using configuration
org-cli list

# List with JSON output
org-cli list --format json

# Search across all configured org files
org-cli search "project planning"

# Search with custom parameters
org-cli search "TODO" --limit 5 --format json --snippet-size 75

# Override root directory for a single command
org-cli --root-directory ~/documents/org search "meeting notes"
```

### Agenda Commands

```bash
# List all tasks (TODO/DONE items)
org-cli agenda list

# List tasks with specific TODO states
org-cli agenda list --states TODO,IN_PROGRESS

# Filter tasks by priority
org-cli agenda list --priority A

# Filter by tags
org-cli agenda list --tags work,urgent

# Show today's scheduled tasks
org-cli agenda today

# Show this week's tasks
org-cli agenda week

# Show tasks in custom date range
org-cli agenda range --start 2025-10-20 --end 2025-10-27

# JSON output for agenda
org-cli agenda list --format json --limit 10
```

### Capture Commands

```bash
# Quick capture into the default notes file
org-cli capture "Review PR #42"

# Capture a TODO with priority and tags
org-cli capture "Fix login bug" \
    --todo-state TODO --priority A --tags work,urgent

# Capture under a target heading (creates missing levels)
org-cli capture "Migrate database" \
    --file projects.org --target-heading "Projects/Backend"

# Capture with planning fields (SCHEDULED, DEADLINE, optional CLOSED)
org-cli capture "Ship v2 release" \
    --scheduled "2026-05-15" \
    --deadline "2026-05-20 17:00"

# Recurring task: SCHEDULED with a repeater (++1w means weekly)
org-cli capture "Weekly review" --scheduled "2026-05-15 ++1w"

# DEADLINE with a 3-day warning lead
org-cli capture "Submit report" --deadline "2026-05-20 -3d"

# Property drawer entries (repeatable; KEY=VALUE)
org-cli capture "Quarterly planning" \
    --property "CATEGORY=planning" \
    --property "EFFORT=2h"

# Datetree journaling: lands under <today's> Year/Month/Day under "Logs"
org-cli capture "Standup notes" \
    --file journal.org --target-heading Logs --datetree

# Backfill an entry under a specific past day
org-cli capture "Retro reflection" \
    --file journal.org --datetree --datetree-date 2026-04-01

# Body content via --body
org-cli capture "Idea" --body "Use a Bloom filter for dedup."
```

Timestamp grammar (used by `--scheduled`, `--deadline`, `--closed`):

```
YYYY-MM-DD [HH:MM] [REPEATER] [WARNING]
```

- Repeater: `+N{u}`, `++N{u}`, or `.+N{u}` where `N` is a positive integer and
  `u ∈ {h, d, w, m, y}` (e.g., `++1w`, `.+3m`).
- Warning: `-N{u}` (e.g., `-3d`).

CLOSED renders with inactive brackets `[...]`; SCHEDULED and DEADLINE use
active brackets `<...>`. The day-of-week abbreviation is added automatically.

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

### Pre-built Binaries

Download the latest pre-built binaries from [GitHub Releases](https://github.com/szaffarano/org-mcp-server/releases/latest):

```bash
# Download org-cli
curl -LO https://github.com/szaffarano/org-mcp-server/releases/latest/download/org-cli-x86_64-unknown-linux-gnu.tar.gz
tar xzf org-cli-x86_64-unknown-linux-gnu.tar.gz
sudo mv org-cli /usr/local/bin/

# Download org-mcp-server
curl -LO https://github.com/szaffarano/org-mcp-server/releases/latest/download/org-mcp-server-x86_64-unknown-linux-gnu.tar.gz
tar xzf org-mcp-server-x86_64-unknown-linux-gnu.tar.gz
sudo mv org-mcp-server /usr/local/bin/
```

Pre-built binaries are available for multiple platforms. Check the [releases page](https://github.com/szaffarano/org-mcp-server/releases) for all available downloads.

### Cargo Install

Install from [crates.io](https://crates.io) using Cargo:

```bash
# Install CLI tool
cargo install org-cli --locked

# Install MCP server
cargo install org-mcp-server --locked
```

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
        "ORG_ORG__ORG_DIRECTORY": "/path/to/your/org/files",
        "ORG_LOGGING__LEVEL": "info",
        "ORG_SERVER__MAX_CONNECTIONS": "20"
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
- [x] Agenda-related Functionality
- [ ] Link following and backlink discovery (org-roam support)
- [ ] Metadata caching for performance

### Phase 3: Extended Capabilities 🚧

- [x] Content creation via `org-capture` (heading + planning + properties + datetree)
- [ ] Content modification (edit existing TODOs: state changes, planning updates,
      property updates, CLOCK / LOGBOOK entries)
- [ ] Media file reference handling
- [ ] Integration with org-roam databases
- [ ] Real-time file watching and updates
- [ ] Advanced query language

## License

[MIT License](LICENSE) - see LICENSE file for details.
