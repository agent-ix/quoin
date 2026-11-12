---
id: FR-040
title: "Assurance case as a read-only view over the store"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
---

# FR-040: Assurance case as a read-only view over the store

## Description

**A pile of green evidence is not an argument.** A matrix of ✅ says every row has something attached
to it; it does not say why the thing anyone actually cares about is true.

The internal structured argument view does: a **claim** is supported by explicit reasoning over
sub-claims, whose leaves are **evidence**. *"StR-004 holds because these
requirements refine it, each verified by a stated method, with evidence that is fresh."*

This is a project-owned representation. It borrows the vocabulary of a goal-structured argument
(goal, strategy, solution) and nothing more: a **goal** is a claim or sub-claim, a **strategy** is the
node that names how a goal is decomposed, and a **solution** is an obligation leaf. It is not GSN. It
does not conform to the GSN standard's node set (no context, assumption, justification or module
nodes), does not read or write GSN interchange, and does not claim that a rendered case is a GSN
diagram. Its markdown and mermaid output are quoin's own formats.

`quoin assurance` renders that from data the program already holds — the trace graph, the obligations
`quire` mints, and the auditor's verdict on each binding. It is **strictly a view**: it runs nothing,
collects nothing, and writes nothing to the store.

### A decomposition names how it is argued

A goal whose sub-claims are found by inverting the corpus's edges gets one **strategy** node between
it and those sub-claims. The strategy names the edge verbs that carry the argument, grouped by verb
and sorted: ``argued over `traces_to`: FR-001, FR-002``. A sub-claim that declares two verbs toward
one parent appears once, under both. A goal with only obligations and no sub-claims has no strategy.

### A supported leaf says what it rests on

A solution leaf that has a binding lists its bound suites, the newest run or scan timestamp the store
holds for each, the newest across them, and the auditor's verdict in the test matrix's terms (FR-115):
`bound`, `stale`, `suspect` or `undischarged`. `bound` means the audit lists the obligation in its
`healthy` set, which is not the same as "no finding": a ladder that stops early (an obligation bound
only to scans) or leaves a check unevaluated raises no finding and is still not healthy. Such a leaf is
open, with the unevaluated check's reason, or a statement that the audit reached no verdict, as its
reason. `suspect` and `stale` come from the findings by the matrix's precedence. "Newest" is the later
instant, compared as RFC 3339 instants rather than as strings. The verdict is read from what `audit()`
produced, never recomputed from timestamps. The `--json` payload carries the same fields.

### Gaps are nodes, not omissions

This is the whole reason to build it, and the reason it is dangerous to build carelessly.

An undischarged obligation, a vacuous binding, a claim nothing traces to — each stays in the tree as
an **open node**, carrying the auditor's reason verbatim. A case that
narrows to only what it can prove is indistinguishable, to a reader, from a case that is complete.

An assurance case is the single most tempting artefact in the system to quietly narrow, because the
narrowed version looks better and no check anywhere would object. So the view's contract is the
opposite: **a claim with no sub-claim and no obligation is `open`**, not vacuously supported, and a
requirement carrying obligations that no claim reaches gets its own section rather than being left
out of the drawing.

That section earned itself on the first run. Over this repository it reported **15 requirements
reachable from no claim**, of which seven had been added during this program and eight predate it
(`agent-ix/quoin#136`).

### The auditor's verdict, not a second opinion

Whether an evidence leaf is supported comes from `audit()` (FR-032). Re-deriving freshness or
suspicion here would be a second answer to a question that already has one, and the two would
disagree the first time either changed.

### The decomposition is read in the direction the corpus writes it

Documents declare their own upward edges — an FR says it `traces_to` an StR, never the reverse — so
the parent→child map is built by inverting what each child declares. The verbs are the corpus's:
`traces_to`, `refines`, `implements`, `derives_from`, `mitigates`, `satisfies`.

An obligation's owner is derived from its **id** (`FR-001-AC-3` → `FR-001`) rather than from an edge.
Obligations are minted from a document's own tables, so the ownership is not in question, and an edge
for it would be a second thing to keep in agreement.

### The claim type is a parameter, never a constant

`StR` is the default. A safety bundle argues from a declared **hazard** and a security bundle from a
**threat**, and that vocabulary is module data — a built-in `StR` would make this view useless to
exactly the bundles that need an assurance case most.

### A shared sub-claim belongs to both claims

Cycle prevention and "already rendered somewhere" are different questions, and one visited-set cannot
answer both. Sharing it put a requirement refining two claims under the first only, and made the
second report *"no sub-claim and no obligation traces to this claim"* — false, in an assurance case,
about the edge its author had written (`SR-007` FND-001). Cycles are prevented per **path**; being
rendered elsewhere is not a reason to omit.

### Deterministic, because it is a view

Re-rendering unchanged inputs produces byte-identical output, so a diff means the evidence moved and
not that somebody re-ran the command. The mermaid graph obeys three authoring rules, each of which
otherwise yields a diagram that **fails to draw** rather than one that draws wrongly: ids are
sanitised (`FR-001-AC-1` reads as an edge fragment), labels are quoted (a `(` in a statement ends the
node shape early), and `;` is replaced (it terminates the statement).

A fourth rule governs the label's length rather than its punctuation, and it fails the other way —
quietly. Labels are truncated on a **code-point** boundary, not on a UTF-16 code unit. Truncating by
code unit splits a surrogate pair, and the result is a string that is not well-formed UTF-16: it
draws, and it draws a replacement character. Measured over this repository's own 991 obligations, 835
labels are truncated and two carry an astral character, the nearest sitting six characters short of
the boundary — so this was a defect waiting on one spec edit rather than a hypothetical
(`agent-ix/quoin#432`).

### Reading the bundle

The upward edges live in each document's frontmatter, and no `quire` surface exposes the bundle graph
as data — `coverage --json` carries obligations with their document, statement and method, and no
relationships. The reader FR-037 introduced is therefore **generalized and shared**, not duplicated:
two readers over the same files drift, and the second gets written by whoever needs a field the first
does not expose. `agent-ix/quire-rs#179` is where this stops being necessary.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-040-AC-1 | The case argues from a top-level claim down through its requirements to their obligations. | Test |
| FR-040-AC-2 | An obligation with an auditor finding stays in the tree as an open node, carrying the reason. | Test |
| FR-040-AC-3 | Open propagates upward: a claim with a broken branch is not reported as supported. | Test |
| FR-040-AC-4 | A claim with no sub-claim and no obligation is open, not vacuously supported. | Test |
| FR-040-AC-5 | Requirements carrying obligations that no claim reaches are reported, not omitted. | Test |
| FR-040-AC-6 | The claim type comes from the caller; a declared hazard argues as readily as an `StR`. | Test |
| FR-040-AC-7 | An obligation's owning requirement is derived from its id. | Test |
| FR-040-AC-8 | Rendering unchanged inputs is byte-identical. | Test |
| FR-040-AC-9 | Mermaid output survives punctuation in a statement — parentheses, quotes and semicolons. | Test |
| FR-040-AC-10 | A bundle declaring no claim says so, rather than rendering an empty case. | Test |
| FR-040-AC-11 | A requirement refining two claims appears under both. | Test |
| FR-040-AC-12 | A cycle terminates without dropping a legitimately shared child. | Test |
| FR-040-AC-13 | A case with no claims carries a machine-readable `reason` naming the searched claim types — present exactly when `claims` is empty — so the `--json` consumer can tell a clean case from one where nothing was argued. | Test |
| FR-040-AC-14 | `--claim-type` is matched case-insensitively against the authored `type:`, and the flag REPLACES the `StR` default rather than adding to it — stated in its help. | Test |
| FR-040-AC-15 | A mermaid label is truncated on a code-point boundary, so a label whose limit falls inside a surrogate pair stays well-formed rather than ending in a lone surrogate. | Test |
| FR-040-AC-16 | A goal decomposed into sub-claims has a strategy node, in markdown and mermaid, that names the edge verbs and the sub-claims argued over each, groups separated unambiguously; a sub-claim declaring two verbs toward one parent appears once. | Test |
| FR-040-AC-17 | A bound leaf lists its suites, the newest run or scan timestamp for each and overall, and the auditor's `bound`, `stale`, `suspect` or `undischarged` verdict, in markdown and `--json`; a bound suite with no run or scan says so. | Test |
| FR-040-AC-18 | A leaf the audit does not list healthy is open even with no finding, and shows the unevaluated check's reason or that the audit reached no verdict. | Test |
| FR-040-AC-19 | "Newest" run or scan is the later instant: fractional seconds and zone offsets order by the instant they name, not by string. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-040-CON-1 | quoin SHALL NOT collect data for this view. It reads the store and the spec, and reports what is missing (ADR-0011 L1). | Design | Inspection |
| FR-040-CON-2 | quoin SHALL NOT omit an unsupported node. A narrowed case reads as a complete one. | Design | Test |
| FR-040-CON-3 | quoin SHALL NOT re-derive freshness or suspicion. The auditor's verdict is the one used. | Design | Inspection |
| FR-040-CON-4 | quoin SHALL NOT write the rendered case into the repository. It is a view, emitted on stdout. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-032](./FR-032-evidence-auditor.md) (the verdict on each leaf), [FR-030](./FR-030-evidence-store.md) (bindings and runs), [FR-037](./FR-037-declared-vocabulary-completeness.md) (the shared bundle reader)
- **Downstream**: `agent-ix/quoin#136` (the requirements no claim reaches), `agent-ix/quire-rs#179` (a bundle graph surface, which retires the frontmatter reader)
