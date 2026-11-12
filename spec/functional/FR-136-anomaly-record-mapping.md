---
id: FR-136
title: "Anomaly lifecycle check over git history and assessment reverse references"
type: FR
relationships:
  - target: "ix://agent-ix/spec-artifacts-process/US-003"
    type: "implements"
  - target: "ix://agent-ix/spec-artifacts-process/FR-014"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-process/FR-015"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-process/FR-011"
    type: "depends_on"
---

# FR-136: Anomaly lifecycle check over git history and assessment reverse references

## Description

Quoin SHALL own the parts of the `Anomaly` record (declared by `agent-ix/spec-artifacts-process`
FR-014 and FR-015) that no module mapping can express: the `statusHistory` derived from the git
history of the document, the check of its status moves, the missing-module report for hazard links,
and the reverse references from a requirement or hazard to the anomalies that name it. This
requirement defines no archetype, vocabulary or table header; those belong to the module.

### What this requirement does not cover

`Anomaly` is a process artifact type, not an object type, so FR-121 and FR-123 (which map the
sections of object artifacts by their object type's record schema) do not govern it, and this
requirement departs from none of their rules. The keys `status`, `resolution`, `closedBy`,
`safetyImpact` and `assessment` are mapped by the module's own `mappings.yaml`
(spec-artifacts-process FR-011, FR-014, FR-015) and extracted by the quire-rs extractor of
`agent-ix/quire-rs#393`, with `frontmatter` and `typed-table` mapping kinds. Quoin repeats none of
that mapping. Reference resolution and kind checks of the assessment links are quire-rs FR-026 and
FR-040, reported by `quire validate`; Quoin SHALL NOT report them again.

## Rationale

The module cannot hold a typed history of who moved an anomaly and when without duplicating, and
being able to falsify, what git records. It therefore leaves transition legality to the one tool
that can read the history, and quoin is that tool.

## Behavior

### Actors and surface

- The command `quoin anomaly check [<scope>]` SHALL evaluate every `Anomaly` document in the scope
  and print its diagnostics, one per line, and its exit status SHALL be non-zero if any diagnostic
  has severity `error`.
- The command `quoin anomaly refs <id> [--json]` SHALL list the anomalies whose assessment names
  `<id>`.
- `quoin anomaly check` SHALL NOT report a dangling reference or a kind mismatch.
- Extraction of the record from a document is independent of the document's validity: a document that
  fails module validation still yields its record, and its diagnostics are `quire validate`'s.

### `statusHistory`

- Quoin SHALL derive a `statusHistory` list from the git history of the file: one entry per commit
  that changed the value of `status`, in commit order, with `status`, `commit`, `author`,
  `authoredAt`, and the signature verdict git reports for that commit (`signed`, `unsigned` or
  `unknown`). The first entry is the commit that added the file.
- Quoin SHALL NOT derive `status` from `statusHistory`.
- Quoin SHALL NOT take any author or date from a field typed into the document.
- If the file is not under git, then Quoin SHALL report `statusHistory` not applicable and SHALL
  emit no transition diagnostic.

### Diagnostics

Each diagnostic has a `code`, a `severity`, a `reason` and the 1-based `line` of the frontmatter
`status` field in the working-tree document (a commit has no line of its own), and names the
commit or commits involved.

| Code | Severity | Reason | Condition |
|---|---|---|---|
| `anomaly.illegal-transition` | error | `transition` | A consecutive pair of `statusHistory` entries is not a legal move of FR-014, judged with the `resolution` of the later commit when the move is to `closed`. |
| `anomaly.illegal-first-state` | error | `first-state` | The first entry's `status` is not `identified`. |
| `anomaly.reassessment-missing` | error | `reopen` | For a `resolved` to `assessed` move, the text of the `## Safety Impact Assessment` section is identical at the reopen commit and at the commit of the next later move out of `assessed`. |
| `anomaly.missing-module` | error | `spec-objects-safety` | The assessment holds a `Hazard` row and the `hazard` object type is not installed; one per document, never one per row. |

- Quoin SHALL emit `anomaly.illegal-transition` for each illegal pair of `statusHistory` entries.
- If the first entry's `status` is not `identified`, then Quoin SHALL emit `anomaly.illegal-first-state`.
- When a move is `resolved` to `assessed` and the assessment text did not change before the next
  later move out of `assessed`, Quoin SHALL emit `anomaly.reassessment-missing` naming the reopen
  commit.
- If the assessment holds a `Hazard` row and the `hazard` object type is not installed, then Quoin
  SHALL emit one `anomaly.missing-module` for the document.

### Reverse references

- `quoin anomaly refs <id>` SHALL list each anomaly whose `assessment` has an entry whose `target`
  resolves to `<id>`, as `{ anomaly, kind, impact }`, in anomaly id order, one JSON object per
  entry under `--json` in a top-level array, and an empty array when none names it.
- Quoin SHALL include an entry whose `target` is an acceptance-criterion id, and SHALL include no
  entry for a `None` row or for a `target` that resolves to nothing.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-136-AC-1 | A fixture repository whose commits set `status` to `identified`, `assessed`, `resolved`, `closed` yields four `statusHistory` entries in order, each with the commit's author, authored date and a signature verdict, and no author or date typed in the document is read. | Test |
| FR-136-AC-2 | A document that fails module validation (a `closed` anomaly with no `closed_by`) still yields its record, and `quoin anomaly check` reports only lifecycle diagnostics for it, not the missing field. | Test |
| FR-136-AC-3 | A history `identified`, `resolved` yields `anomaly.illegal-transition` naming both commits; `assessed`, `closed` with `resolution: fixed` yields one; `identified`, `closed` with `duplicate` yields none; `resolved`, `assessed` yields none. | Test |
| FR-136-AC-4 | A history whose first entry is not `identified` yields `anomaly.illegal-first-state`. | Test |
| FR-136-AC-5 | A history `resolved`, `assessed`, `resolved` with the assessment section unchanged yields `anomaly.reassessment-missing` naming the reopen commit, and the same history with an edited assessment section yields none. | Test |
| FR-136-AC-6 | A fixture file outside git reports `statusHistory` not applicable and no transition diagnostic. | Test |
| FR-136-AC-7 | For a corpus with a requirement `FR-001`, its criterion `FR-001-AC-1` and a hazard, `quoin anomaly refs` on each lists the anomaly that names it with its `Impact`; on a hazard no anomaly names it prints `[]` under `--json`; an anomaly whose only row is a `None` row appears in no listing. | Test |
| FR-136-AC-8 | An anomaly with a `Requirement` row targeting `FR-999` is absent from every listing, and `quoin anomaly check` emits no dangling-reference diagnostic for it. | Test |
| FR-136-AC-9 | The anomaly documents the tests use are those shipped by `agent-ix/spec-artifacts-process` FR-014 and FR-015; the commit sequences are authored by this repository's tests, and the quoin repository contains no copy of the documents. | Inspection |
| FR-136-AC-10 | With `spec-objects-safety` not installed, an anomaly with two `Hazard` rows yields exactly one `anomaly.missing-module`. | Test |
| FR-136-AC-11 | Every diagnostic in AC-3 to AC-5 and AC-10 carries its code, severity `error`, its reason, and the line of the document's `status` field. | Test |

## Dependencies

- **Upstream**: `agent-ix/spec-artifacts-process` [FR-011](ix://agent-ix/spec-artifacts-process/FR-011), [FR-014](ix://agent-ix/spec-artifacts-process/FR-014) and [FR-015](ix://agent-ix/spec-artifacts-process/FR-015) (the document shape, the record mapping, the fixtures)
- **Upstream, quire-rs**: [FR-026](ix://agent-ix/quire-rs/FR-026) (reference resolution), [FR-040](ix://agent-ix/quire-rs/FR-040) (edge target typing), and the artifact-type record extractor `agent-ix/quire-rs#393`
- **Downstream**: the anomaly views and reports that read the record
