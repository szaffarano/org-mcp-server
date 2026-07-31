---
name: org-search
description: >
  Search and browse org-mode documents via the org MCP server. Use when the user wants to
  find content across org files, list their org documents, read file contents, or navigate
  headings and outlines.
  Triggers: "search my notes for...", "find org pages about...", "list my org files",
  "show outline of...", "read the section about...", "find documents tagged with...",
  "what files do I have about...", or any request to search, browse, read, or navigate
  org-mode documents and their content.
---

# Org Search

Search and browse org-mode documents using the `@org` MCP server.

## Tools

### Search — `mcp__org__org-search`

Fuzzy text search across all org files.

**Parameters:**
- `query` (required): Search string
- `tags`: Filter results by tags
- `limit`: Max results
- `snippet_max_size`: Snippet length in chars (default: 100). Use 300+ for more context.

### File List — `mcp__org__org-file-list`

List org files in the configured directory.

**Parameters:**
- `tags`: Filter by tags
- `limit`: Max files

## Resources

Use `ReadMcpResourceTool` with `server: "org"` to access file content:

| URI pattern | Use case |
|---|---|
| `org://` | List all org files |
| `org://{file}` | Read full file content |
| `org-outline://{file}` | Get heading structure (table of contents) |
| `org-heading://{file}#{heading}` | Read a specific heading's content |
| `org-id://{uuid}` | Access a node by its unique ID |

## Workflow

1. **Search for content**: Use `mcp__org__org-search` with the user's query. If results are
   too broad, suggest narrowing with tags or more specific terms.
2. **Browse files**: Use `mcp__org__org-file-list` to list available files, optionally
   filtered by tags.
3. **Explore structure**: Read `org-outline://{file}` to see a file's heading hierarchy
   before diving in.
4. **Read content**: Use `org://{file}` for full content or `org-heading://{file}#{heading}`
   to read a specific section.
5. **Present results**: Show file name, matched heading, and relevant snippet. Link to
   deeper content when available.
