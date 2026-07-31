---
name: org-capture
description: >
  Create new org-mode headings and notes via the org MCP server. Use when the user wants
  to add a new task, note, journal entry, or any new heading to their org files.
  Triggers: "add a task", "create a note", "capture this", "log an entry",
  "add to my notes", "create a TODO", "add a journal entry", "record this idea",
  or any request to write new content into org files.
---

# Org Capture

Append new headings and notes to org files using the `mcp__org__org-capture` tool.

## Tool — `mcp__org__org-capture`

**Required:**
- `title`: Heading text. Non-empty, no newlines.

**Targeting (where to insert):**
- `file`: Relative path within the org directory. Defaults to the configured notes file.
- `target_heading`: Slash-separated path to insert under (e.g. `"Projects/Work"`).
  Missing intermediate headings are created automatically.

**Heading metadata:**
- `level`: Heading depth (1–19). Defaults to `parent_level + 1` when `target_heading` is
  set, else 1.
- `todo_state`: Keyword matching `org_todo_keywords` config (e.g. `"TODO"`, `"DONE"`).
- `priority`: `"A"`, `"B"`, or `"C"`.
- `tags`: List of tag strings. Each must match `^[A-Za-z0-9_@]+$`.
- `body`: Text placed beneath the heading.
- `properties`: List of `{key, value}` pairs written into a property drawer.

**Timestamps** (ISO `YYYY-MM-DD` or `YYYY-MM-DD HH:MM`, optional repeater/warning):
- `scheduled`: SCHEDULED active timestamp.
- `deadline`: DEADLINE active timestamp.
- `closed`: CLOSED inactive timestamp.
- Repeater syntax: `+N`, `++N`, or `.+N` followed by `h|d|w|m|y`.
- Warning syntax: `-N` followed by `h|d|w|m|y`.

**Datetree** (Year/Month/Day hierarchy):
- `datetree: true`: Expand `target_heading` with a Year → Month → Day tree before
  resolving, placing the entry under today's leaf.
- `datetree_date`: Override the target day (`YYYY-MM-DD`). Only valid when `datetree` is
  true.

## Common patterns

| Goal | Key parameters |
|---|---|
| Quick TODO | `title`, `todo_state: "TODO"` |
| Scheduled task | `title`, `todo_state`, `scheduled` |
| Journal entry today | `datetree: true`, `title`, `body` |
| Journal entry specific day | `datetree: true`, `datetree_date: "YYYY-MM-DD"`, `title` |
| Note under a heading | `target_heading: "Area/Subarea"`, `title`, `body` |
| Note in specific file | `file: "relative/path.org"`, `title` |
| Tagged note | `title`, `tags: ["tag1", "tag2"]` |
| Note with properties | `title`, `properties: [{"key": "SOURCE", "value": "..."}]` |

## After capture

The tool returns the file path and the position of the newly created heading. Use
`org-heading://{file}#{title}` to verify the result if needed.
