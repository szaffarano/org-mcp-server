# Installing org-mcp-server skills for OpenCode

## Prerequisites

- [OpenCode.ai](https://opencode.ai) installed
- The `org` MCP server configured in your OpenCode setup

## Installation

Add org-mcp-server to the `plugin` array in your `opencode.json`
(global or project-level):

```json
{
  "plugin": ["org-mcp-server@git+https://github.com/szaffarano/org-mcp-server.git"]
}
```

Restart OpenCode. The plugin registers the `org-agenda` and `org-search` skills.

## Skills

| Skill | Triggers |
|---|---|
| `org-agenda` | "show my agenda", "tasks for today", "what's overdue", "my TODOs" |
| `org-search` | "search my notes for…", "list my org files", "show outline of…" |

## Usage

Use OpenCode's native `skill` tool to list or load skills:

```
use skill tool to list skills
use skill tool to load org-agenda
```

## Updating

To pin a specific version:

```json
{
  "plugin": ["org-mcp-server@git+https://github.com/szaffarano/org-mcp-server.git#v0.1.0"]
}
```

## Troubleshooting

### Skills not found

1. Verify the plugin line is in your `opencode.json`
2. Check logs: `opencode run --print-logs "hello" 2>&1 | grep -i org`
3. Make sure the `org` MCP server is running and configured
