---
name: org-agenda
description: >
  View and manage org-mode agenda, scheduled tasks, TODOs, and deadlines via the org MCP server.
  Use when the user asks about their schedule, agenda, tasks, or deadlines.
  Triggers: "show my agenda", "what's on my schedule", "tasks for today", "tomorrow's agenda",
  "what do I have this week", "show next week", "last week's tasks", "my TODOs",
  "show deadlines", "high priority tasks", "what's overdue", or any request about
  scheduled items, TODO states, or calendar-like task views.
---

# Org Agenda

View and manage org-mode agenda items using the `@org` MCP server.

## Tool — `mcp__org__org-agenda`

Query agenda items with filtering and two display modes.

**Parameters:**
- `mode`: `"view"` (calendar-like, organized by date) or `"list"` (flat task list, default)
- `start_date` / `end_date`: ISO 8601 (`YYYY-MM-DD`). Compute from relative terms.
- `todo_states`: e.g. `["TODO"]`, `["DONE"]`, `["TODO", "IN_PROGRESS"]`
- `priority`: `"A"`, `"B"`, or `"C"`
- `tags`: e.g. `["work", "urgent"]`
- `limit`: Max items to return

**When to use each mode:**
- `"view"` — Date-range queries (today, tomorrow, this week, last month, etc.)
- `"list"` — Broad task listings without a specific date range (all TODOs, all done items)

**Common patterns:**

| Request | Parameters |
|---|---|
| Today's agenda | `mode: "view"`, both dates = today |
| Tomorrow | `mode: "view"`, both dates = tomorrow |
| This week | `mode: "view"`, start = Monday, end = Sunday |
| Next week | `mode: "view"`, start = next Monday, end = next Sunday |
| Last week | `mode: "view"`, start = prev Monday, end = prev Sunday |
| All open tasks | `mode: "list"`, `todo_states: ["TODO"]` |
| Done items | `mode: "list"`, `todo_states: ["DONE"]` |
| High priority | add `priority: "A"` to any query |
| Tagged items | add `tags: ["tag"]` to any query |

## Priority filter caveat

`priority` filtering only works in `"list"` mode. It is silently ignored in `"view"` mode —
use `"list"` mode and add a date-range filter manually when you need both.

## Resources

Use `ReadMcpResourceTool` with `server: "org"` for quick snapshots without filtering:

| URI | Content |
|---|---|
| `org-agenda://` | All agenda items |
| `org-agenda://today` | Today's scheduled items |
| `org-agenda://week` | This week's items |
| `org-agenda://day/{YYYY-MM-DD}` | Agenda for a specific day |
| `org-agenda://week/{N}` | Agenda for week number N |
| `org-agenda://month/{N}` | Agenda for month number N |
| `org-agenda://query/from/{YYYY-MM-DD}/to/{YYYY-MM-DD}` | Custom date range |

Prefer the tool over resources when filtering by priority, tags, states, or custom date ranges.

## Presentation

- For `"view"` mode results, group items by date with clear date headers.
- For `"list"` mode results, group by state or priority.
- Highlight overdue items and upcoming deadlines.
