# ManualForge Backlog

Filed work not yet built. Newest first. This is a filing + spec-sketch
backlog, not a build queue — each entry carries its demand evidence and a
sketch deep enough that a future wave can scope it without re-researching.

---

## MF-001 — Highlight-a-passage-and-ask, inline (Karpathy gap)

**Status:** filed 2026-10-10. **Priority:** high — the market gap is named by
name in demand evidence below.

**Demand evidence (Karpathy, "How I use LLMs", 2h11m, verbatim captions):**
> "It is rarely ever the case anymore that I read books just by myself —
> I always involve an LLM," chapter-by-chapter: paste chapter, get summary,
> read, ask questions as they arise.
>
> And the tool he wants *doesn't exist* — highlight-a-passage-and-ask,
> inline, without the clunky copy-paste round-trip.

That is two facts in one quote: (1) reading-with-an-LLM is already his
settled habit — the behavior is proven, not speculative; (2) the friction
point he names is precise: the copy-paste round-trip between the text and
the chat. A tool that removes exactly that friction has a pre-verified
user.

**Why ManualForge owns this:** ManualForge's thesis is that manuals are
graphs with addressable sections (`mf` law: numbered addresses, stable,
every section reachable by API/MCP/human eyes). A reader highlighting
passage §4.2.3 and asking a question is just an edge being created —
`(question) -asks-about-> (section 4.2.3)` — and the ManualForge index/
reader (`mf index` → `index.json` + `reader.html`, self-contained, zero
deps) is already the surface where that edge should be drawable. The
read-with lane pattern (`~/workspace/full-send/patterns/read-with-pattern.md`)
is the machine-side doctrine; this tool is the human-side surface.

**Spec sketch (not a build):**
- Reader UI (`reader.html`): select text → inline "ask" affordance →
  question attached to the nearest numbered section address (not to a
  page offset — addresses are stable, offsets aren't).
- Ask routes to the configured model with the passage + its section
  context (parent/child/xref edges from `graph.json`) as the prompt
  ground — the model answers about the passage *in place*, never a chat
  sidebar.
- Q&A pairs persist as edges in `graph.json` (append-only; questions are
  addressable, citable, and feed back into the manual as living FAQ).
- `mf check` extends to the new edge type: every question resolves to a
  real section address.
- First surface: the existing `reader.html` (zero deps, ships free). No
  new auth, no new infra.

**Demand risks (honest):** Karpathy is one data point, however pointed.
Before building: confirm 2–3 more readers (his pedicab-operator use case
— reading dense regulatory/funding docs on the road — counts as one) hit
the same copy-paste friction. If confirmed, this graduates from backlog
to wave scope.

**Related:** WO-READWITH read-with lane pattern + brief template (the
machine-side companion pattern); lensA-novel-how-i-use-llms.md #5.
