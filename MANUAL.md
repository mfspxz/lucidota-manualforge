# 1 Overview

ManualForge (`mf`) turns a `MANUAL.md` file in a repo root into a living
manual: numbered sections that are individually addressable by humans, HTTP
APIs, and machine agents. One source file, many views, zero drift. See → 2
for the law the manuals obey and → 4 for the commands.

## 1.1 What it is

A Rust binary that parses Law-of-Root manuals (→ 2) into a section graph
(`graph.json`), a human reader (`index.html`), and a JSON API. It is the
reference implementation of the Document Forge. Details: → 3.

## 1.2 Status

Production tool, in daily use. The research manuals for the Notion
organization project were written, checked, and built with it. See → 8.

# 2 The Law

Every ManualForge manual obeys five conventions, modeled on the Law of Root
(Cole Wehrle). The law is what makes manuals machine-readable. Details per
rule: → 2.1 – → 2.5.

## 2.1 Numbered address

Every section has a dotted address: `1`, `2.3`, `4.1.2`. Addresses are
stable. Never renumber — append new sections, mark old ones superseded with
a pointer to the replacement section.

## 2.2 One claim per section

A section states one thing and its evidence. Subsections carry the detail.
If a section needs "and", it needs splitting.

## 2.3 Citations, not repetition

Reference another section with `→ 2.3` instead of restating it. The graph
resolves the link in every view. Repetition rots; citations stay fresh.

## 2.4 Every view from one graph

`graph.json` is the source of truth. `index.html` (human), `/api/section/{id}`
(API), and the MCP tools (→ 5) all read it. There is no second copy to
go stale.

## 2.5 Machine-checkable

`mf check` verifies: addresses unique, parents exist, every `→ xref`
resolves, no orphan sections (except `1`). A manual that fails `mf check`
is not a manual yet.

# 3 Source format

`MANUAL.md` in the repo root. Headings carry the address — a level-1
heading reads `# 1 Overview`, a level-2 subsection reads `## 1.1 What it
does`, and so on down to `#### 1.1.1.1` for four levels. Body text follows
the heading; cross-references use the `→ 2.3` form.

`mf init <repo>` scaffolds this from the repo itself: README → §1,
install/usage → §2–3, source layout → §4, deps → §5, plus §6 config,
§7 troubleshooting, §8 changelog. See → 4.1.

# 4 Tool commands

Six commands cover the whole lifecycle: scaffold, build, check, serve,
query, index. Details: → 4.1 – → 4.6.

## 4.1 mf init

`mf init <repo-dir>` scans a repo and scaffolds `MANUAL.md`.
`mf init external:<name>` scaffolds the standardized third-party manual:
§1 overview + official docs link, §2 why we use it, §3 how we use it,
§4 compatibility notes + version pin, §5 references.

## 4.2 mf build

`mf build` compiles `MANUAL.md` into `manual/graph.json`,
`manual/sections.json`, and `manual/index.html`. Run it after every edit.

## 4.3 mf check

`mf check` verifies the law (→ 2.5): unique addresses, resolved xrefs,
no orphans. Run before every commit; gate releases on it.

## 4.4 mf serve

`mf serve [port]` starts HTTP: `/` (human), `/api/sections`,
`/api/section/{id}`, `/api/graph`, `/api/search?q=`. In a directory with
`index.json` (→ 4.6) it serves the multi-manual reader instead.

## 4.5 mf mcp

`mf mcp` runs an MCP server on stdio. Any MCP-capable agent reads any
section by address without knowing the file layout. Tools: → 5.

## 4.6 mf index

`mf index <repos-root>` scans every `MANUAL.md` below root into
`index.json` (all sections, repo-scoped) plus a self-contained
`reader.html` (zero dependencies). One reader for all manuals.

# 5 MCP tools

The stdio server (→ 4.5) exposes four tools: `get_section(id)` →
{id, title, body, parent, children, xrefs}; `list_sections` → [{id, title,
level}]; `search_sections(query)` → ranked [{id, title, snippet}];
`get_children(id)` → child sections. This is the machine-agent read path.

# 6 Views

Three views, one graph (→ 2.4). Human: `index.html` — TOC sidebar, deep
links (`#2.3`), xref links resolve. API: JSON per section; `/api/graph`
gives nodes + edges for graph UIs. MCP: agents read by address (→ 5).
Notion: the manuals also live as a hub + sections database — see → 7.

# 7 Running the forge on Notion

The forge runs ON Notion through a split-brain loop, because Notion
exposes no public text-generation or code-execution endpoint: Notion is
the readable/writable surface, this VM is the build engine. The full
procedure is the runbook on the Document Forge hub page in Notion.

## 7.1 Notion as the read/write surface

Every manual section is a row in the Forge Sections database (Address,
Manual, Level, Parent, Xrefs, body as page content). A Notion agent
navigates the whole graph through Notion MCP search and fetch — no file
access needed to read.

## 7.2 The VM as the build engine

`mf build`, `mf check`, and `mf mcp` run on this VM against the
`MANUAL.md` sources in `~/workspace/notion-org/research/`. Generation
uses the operator's own providers; Notion only stores the result.

## 7.3 The sync loop

Edits flow: Notion section row → export script writes `MANUAL.md` →
`mf check` → `mf build` → import script refreshes the database. Reads
flow: Notion search/fetch, or `mf mcp` for file-side agents. Scripts live
in `~/workspace/notion-org/bin/`.

# 8 Changelog

## 8.1 Unreleased

- Manual rewritten from SPEC.md to real content; forge adopted as the
  documentation standard for the Notion organization project.
- Notion operation added (→ 7): hub page, Forge Sections database,
  Notion-agent runbook.
