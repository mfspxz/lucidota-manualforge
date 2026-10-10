# ManualForge — Law-of-Root Manuals for Every Repo

Every repo gets a **living manual** modeled on the Law of Root (Cole Wehrle):
numbered sections (`1`, `1.1`, `1.1.1`), each one individually addressable —
by API, by MCP, and by human eyes — with cross-references (`→ 3.2`) that
resolve like law citations. The manual is a **graph first**: sections are
nodes, parent/child and see-also links are edges. Markdown, HTML, JSON, and
MCP are just views over the same graph.

## The Law (conventions)

1. **Numbered address.** Every section has a dotted address: `1`, `2.3`, `4.1.2`.
   Addresses are stable. Never renumber — append new sections, mark old ones
   superseded with a pointer (`→ 5.4`).
2. **One claim per section.** A section states one thing and its evidence.
   Subsections carry the detail.
3. **Citations, not repetition.** Reference another section with `→ 2.3`
   instead of restating it. The graph resolves the link in every view.
4. **Every view from one graph.** `graph.json` is the source of truth.
   `index.html` (human), `/api/section/{id}` (API), and MCP tools all read it.
5. **Machine-checkable.** `mf check` verifies: addresses unique, parents exist,
   every `→ xref` resolves, no orphan sections (except `1`).

## Source format

`MANUAL.md` in the repo root. Headings carry the address:

```markdown
# 1 Overview
Body text. Details live in → 1.1.

## 1.1 What it does
More detail.
```

`mf init <repo>` scaffolds this from the repo itself: README → §1,
install/usage → §2–3, source layout → §4, deps → §5, plus §6 config,
§7 troubleshooting, §8 changelog.

## Tool

```
mf init <repo-dir>    scan repo, scaffold MANUAL.md
mf build              MANUAL.md → manual/graph.json + manual/index.html
mf check              verify the law (unique addrs, resolved xrefs, no orphans)
mf serve [port]       HTTP: / (human)  /api/sections  /api/section/{id}
                      /api/graph  /api/search?q=
mf mcp                MCP server on stdio: get_section, list_sections,
                      search_sections, get_children
```

## MCP tools

- `get_section(id)` → {id, title, body, parent, children, xrefs}
- `list_sections` → [{id, title, level}]
- `search_sections(query)` → ranked [{id, title, snippet}]
- `get_children(id)` → child sections

## Views

- **Human:** `index.html` — TOC sidebar, deep links (`#2.3`), xref links resolve.
- **API:** JSON per section; `/api/graph` gives nodes + edges for graph UIs.
- **MCP:** any agent reads any section by address.

## Index & reader (2026-10-10)

- `mf index <repos-root>` scans every `MANUAL.md` below root → `index.json`
  (all sections, repo-scoped) + `reader.html` (self-contained, zero deps).
- `mf serve` in a directory with `index.json` becomes the **reader**: `/`
  serves the reader UI (manual switcher, full-text search across ALL manuals),
  `/api/index` serves the whole graph. One app that reads everything.
- `mf init external:<name>` scaffolds the standardized third-party manual:
  §1 overview + official docs link, §2 why we use it, §3 how we use it,
  §4 compatibility notes + version pin, §5 references. Their docs, our law.
