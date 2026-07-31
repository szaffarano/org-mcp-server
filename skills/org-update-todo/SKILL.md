---
name: org-update-todo
description: >
  Update the TODO state and metadata of an existing org-mode heading via the org MCP server.
  Use when the user wants to change a task's state, priority, tags, or timestamps.
  Triggers: "mark as done", "complete this task", "reschedule", "change priority",
  "update the deadline", "close this TODO", "set a deadline", "retag this", "move to IN_PROGRESS",
  or any request to modify an existing task or heading's metadata.
---

# Org Update Todo

Update TODO state and planning metadata of existing headings using `mcp__org__org-update-todo`.

## Tool — `mcp__org__org-update-todo`

### Targeting (one of these is required)

**By ID (preferred):** supply `id` — the `:ID:` property value of the heading.
Wins over file/heading_path when both are given.

**By location:** supply both `file` (relative path within org directory) and
`heading_path` (slash-separated path, e.g. `"Projects/Work/Task name"`).

### Fields to update (all optional — omit what you don't want to change)

- `todo_state`: New keyword matching `org_todo_keywords` (e.g. `"TODO"`, `"DONE"`).
  `CLOSED` is auto-stamped on done transitions and auto-removed on reactivation unless
  `org_auto_closed_timestamp` is disabled in config.
- `priority`: `"A"`, `"B"`, or `"C"`.
- `tags`: Replaces the tag list wholesale. Each must match `^[A-Za-z0-9_@]+$`.
  Pass an empty list `[]` to remove all tags.

**Timestamps** (ISO `YYYY-MM-DD` or `YYYY-MM-DD HH:MM`, optional repeater/warning):
- `scheduled`: New SCHEDULED active timestamp.
- `deadline`: New DEADLINE active timestamp.
- `closed`: New CLOSED inactive timestamp (usually auto-managed; set manually only when
  overriding auto-close behavior).
- Repeater syntax: `+N`, `++N`, or `.+N` followed by `h|d|w|m|y`.
- Warning syntax: `-N` followed by `h|d|w|m|y`.

- `title`: Replace the heading title. Non-empty, no newlines.
- `body`: Replace the entire body text. Pass empty string or
  `clear: ["body"]` to remove. Setting and clearing in the same call is an error.
- `properties`: List of `{key, value}` pairs to upsert into the property drawer.
  Keys are case-insensitive; new keys are appended, existing keys are updated.
- `remove_properties`: List of property keys to delete from the drawer.
  Removing a non-existent key is a no-op. A key must not appear in both
  `properties` and `remove_properties`.

**Clearing fields:**
- `clear`: List of field names to remove entirely.
  Valid values: `"todo_state"`, `"priority"`, `"tags"`, `"scheduled"`, `"deadline"`,
  `"closed"`, `"body"`.

## Common patterns

| Goal | Key parameters |
|---|---|
| Mark done | `id` or `file`+`heading_path`, `todo_state: "DONE"` |
| Start working | `todo_state: "IN_PROGRESS"` |
| Reopen a task | `todo_state: "TODO"`, `clear: ["closed"]` |
| Raise priority | `priority: "A"` |
| Reschedule | `scheduled: "YYYY-MM-DD"` |
| Set deadline | `deadline: "YYYY-MM-DD"` |
| Remove deadline | `clear: ["deadline"]` |
| Retag | `tags: ["new_tag1", "new_tag2"]` |
| Remove all tags | `tags: []` |
| Rename a task | `title: "New name"` |
| Replace body text | `body: "Updated description."` |
| Remove body text | `clear: ["body"]` |
| Set effort estimate | `properties: [{"key": "EFFORT", "value": "2h"}]` |
| Set category | `properties: [{"key": "CATEGORY", "value": "work"}]` |
| Remove a property | `remove_properties: ["EFFORT"]` |

## Workflow

1. **Find the target**: Use `mcp__org__org-search` or `mcp__org__org-agenda` to locate the
   heading. Prefer using its `:ID:` property if available.
2. **Apply the update**: Call `mcp__org__org-update-todo` with the target and desired fields.
3. **Confirm**: The tool returns the updated heading state. Verify the result matches intent.
