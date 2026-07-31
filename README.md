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
- `org-agenda://day/{YYYY-MM-DD}` — Agenda for a specific day
- `org-agenda://week/{N}` — Agenda for week number N
- `org-agenda://month/{N}` — Agenda for month number N
- `org-agenda://query/from/{YYYY-MM-DD}/to/{YYYY-MM-DD}` — Custom date range

### MCP Tools

- `org-file-list` — List all org files in configured directories
- `org-search` — Full-text fuzzy search across all org files
- `org-agenda` — Query agenda items with filtering by dates, states, tags, and priorities
- `org-capture` — Append a heading to an org file. Supports TODO state, priority, tags,
  body, SCHEDULED/DEADLINE/CLOSED timestamps (with repeater and warning suffixes),
  property drawer entries, and datetree expansion.
- `org-update-todo` — Update an existing heading in place. Set or clear todo_state,
  priority, tags, planning timestamps, heading title, body text, and property drawer
  entries (per-key upsert/remove).

## Agent Skills Plugin

This repository ships skills for Claude Code, Codex, Cursor, and OpenCode that guide
agents to use the MCP server effectively.

| Skill | Purpose |
|---|---|
| `org-agenda` | Query agenda items by date, state, priority, and tags |
| `org-search` | Search and browse org files and headings |
| `org-capture` | Create new headings, tasks, and journal entries |
| `org-update-todo` | Update TODO state, title, body, properties, and timestamps |

### Claude Code

```bash
claude plugin install github:szaffarano/org-mcp-server
```

Or add to `.claude/settings.json`:

```json
{ "plugins": ["github:szaffarano/org-mcp-server"] }
```

### Codex / Cursor

Add to your plugin configuration using `.codex-plugin/plugin.json` or
`.cursor-plugin/plugin.json` from this repository.

### OpenCode

```json
{ "plugin": ["org-mcp-server@git+https://github.com/szaffarano/org-mcp-server.git"] }
```

See [`.opencode/INSTALL.md`](.opencode/INSTALL.md) for full setup instructions.

## Setup

### Pre-built Binaries

Download the latest pre-built binaries from [GitHub Releases](https://github.com/szaffarano/org-mcp-server/releases/latest):

```bash
curl -LO https://github.com/szaffarano/org-mcp-server/releases/latest/download/org-mcp-server-x86_64-unknown-linux-gnu.tar.gz
tar xzf org-mcp-server-x86_64-unknown-linux-gnu.tar.gz
sudo mv org-mcp-server /usr/local/bin/
```

Binaries are available for Linux, macOS, and Windows. Check the
[releases page](https://github.com/szaffarano/org-mcp-server/releases) for all targets.

### Cargo

```bash
cargo install org-mcp-server --locked
cargo install org-cli --locked
```

### Nix

```bash
nix run github:szaffarano/org-mcp-server       # run directly
nix profile install github:szaffarano/org-mcp-server  # install to profile
nix develop github:szaffarano/org-mcp-server   # development shell
```

### From Source

```bash
git clone https://github.com/szaffarano/org-mcp-server
cd org-mcp-server
cargo build --release
```

## MCP Server Integration

Add to your AI agent configuration (e.g., `~/.claude.json`, `opencode.json`):

```json
{
  "mcpServers": {
    "org-mode": {
      "command": "/path/to/org-mcp-server",
      "args": [],
      "env": {
        "ORG_ORG__ORG_DIRECTORY": "/path/to/your/org/files"
      }
    }
  }
}
```

Or via Nix:

```json
{
  "mcpServers": {
    "org-mode": {
      "command": "nix",
      "args": ["run", "github:szaffarano/org-mcp-server"]
    }
  }
}
```

## Configuration

Config file: `~/.config/org-mcp/config.toml` (or `$XDG_CONFIG_HOME/org-mcp/config.toml`).

```toml
[org]
org_directory = "~/org/"
org_default_notes_file = "notes.org"
org_agenda_files = ["agenda.org", "projects.org"]
org_todo_keywords = ["TODO", "|", "DONE"]
# Auto-prepend :CREATED: property on capture (default: true)
org_auto_created_property = true
# Auto-stamp CLOSED on done transitions (default: true)
org_auto_closed_timestamp = true

[logging]
level = "info"
file = "~/.local/share/org-mcp-server/logs/server.log"

[cli]
default_format = "plain"  # plain | json
```

Configuration is resolved in order: CLI flags → environment variables (`ORG_` prefix,
`__` separator, e.g. `ORG_ORG__ORG_DIRECTORY`) → config file → defaults.

```bash
org-cli config init   # create default config file
org-cli config show   # display current resolved config
org-cli config path   # show config file location
```

## CLI Tool

`org-cli` mirrors the MCP server's capabilities for scripting and testing.
Run `org-cli --help` or `org-cli <subcommand> --help` for full usage.

Quick examples:

```bash
# Search across all org files
org-cli search "project planning"

# Agenda
org-cli agenda today
org-cli agenda list --states TODO,IN_PROGRESS --tags work

# Capture a TODO with planning
org-cli capture "Fix login bug" --todo-state TODO --priority A \
    --scheduled "2026-05-15" --deadline "2026-05-20 -3d"

# Capture under a datetree
org-cli capture "Standup notes" --file journal.org --target-heading Logs --datetree

# Update an existing heading
org-cli update-todo --id abc123 --todo-state DONE
org-cli update-todo --file projects.org --heading-path "Work/Task" \
    --title "Renamed task" --property "EFFORT=2h"
```

Timestamp grammar for `--scheduled`, `--deadline`, `--closed`:
`YYYY-MM-DD [HH:MM] [+N{h|d|w|m|y} | ++N{u} | .+N{u}] [-N{u}]`

## Architecture

Multi-crate Rust workspace:

- **org-core** — Business logic and org-mode parsing
- **org-mcp-server** — MCP protocol implementation
- **org-cli** — CLI interface for testing and direct usage

Built with [orgize](https://crates.io/crates/orgize) (org-mode parsing),
[rmcp](https://crates.io/crates/rmcp) (MCP protocol),
[tokio](https://crates.io/crates/tokio) (async runtime), and
[nucleo-matcher](https://crates.io/crates/nucleo-matcher) (fuzzy search).

## Development

```bash
cargo test                                              # run all tests
cargo test -p org-core                                  # test specific crate
cargo fmt && cargo clippy --all-targets --all-features  # format and lint
cargo run --bin org-cli -- --help                       # run CLI
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
- [x] Content modification: TODO state, priority, tags, and planning updates via
      `org-update-todo` / `org-cli update-todo`
- [x] Content modification: property drawer updates (upsert/remove individual keys)
- [ ] Content modification: CLOCK / LOGBOOK entries
- [ ] `org-promote` / `org-demote` — relative heading level change
- [ ] `org-clock` — clock in / clock out / cancel clock
- [ ] `org-refile` — move heading to different file or location
- [ ] `org-archive` — archive heading to archive file or toggle ARCHIVE tag
- [ ] Media file reference handling
- [ ] Integration with org-roam databases
- [ ] Real-time file watching and updates
- [ ] Advanced query language

## License

[MIT License](LICENSE) - see LICENSE file for details.
