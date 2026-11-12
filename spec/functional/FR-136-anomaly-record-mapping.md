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

> **CR note (PLAT-1415, temporary exception):** the quire-rs record extractor
> (`agent-ix/quire-rs#393`) does not exist yet, so `quoin-anomaly` reads the five record keys
> (`status`, `resolution`, `closed_by`, `safety_impact` and the assessment table) itself, which
> repeats part of the module mapping this requirement says Quoin repeats none of. The reader is
> marked in `rust/crates/quoin-anomaly/src/record.rs` and is deleted when the extractor lands. It
> makes no vocabulary check: an out-of-vocabulary `Impact` or `status` is `quire validate`'s
> under spec-artifacts-process FR-014/FR-015 and still yields a record; only an unreadable
> structure (a missing key, a wrong table header or row arity, an empty table) is a mapping error.

## Rationale

The module cannot hold a typed history of who moved an anomaly and when without duplicating, and
being able to falsify, what git records. It therefore leaves transition legality to the one tool
that can read the history, and quoin is that tool.

## Behavior

### Actors and surface

The `anomaly` topic is new because no existing surface fits. `quoin graph` (FR-062) analyses a
validated assurance export and the evidence store, not individual authored documents or their git
history, and `quoin validate` checks document shape and does not read commits. The anomaly checks
read the working tree and git together, so they get their own topic.

- The command `quoin anomaly check [<scope>]` SHALL evaluate every `Anomaly` document in the scope
  and print its diagnostics, one per line.
- The exit status of `quoin anomaly check` SHALL be non-zero if any diagnostic has severity `error`.
- The command `quoin anomaly refs <id> [--json]` SHALL list the anomalies whose assessment names
  `<id>`.
- `quoin anomaly check` SHALL NOT report a dangling reference or a kind mismatch, which are
  `quire validate`'s.
- Extraction of the record is independent of schema-only validation failures: a document that
  fails only the module's record-schema checks (a missing `closed_by`) still yields its record. A
  document with a mapping error (spec-artifacts-process FR-011: any mapping error means no record)
  yields no record, and `quoin anomaly check` SHALL report it `anomaly.no-record` with the mapping
  error's code and skip the checks that need the record (the assessment checks), while still
  checking `statusHistory`, which reads the history of the file and not the record.

### `statusHistory`

- Quoin SHALL read the first-parent history of the repository's default branch for the file,
  following renames.
- Quoin SHALL derive a `statusHistory` list from that history: one entry per commit that changed
  the value of `status`, in commit order, with `status`, `commit`, `author`, `authoredAt`,
  `signature` and `signatureCode`. The first entry is the commit that added the file.
- Quoin SHALL set `signatureCode` to the letter git reports for the commit (`G`, `B`, `U`, `X`,
  `Y`, `R`, `E` or `N`) and `signature` from it: `signed` for `G`, `U`, `X` and `Y` (a valid
  signature is present; trust and expiry are not judged here), `unsigned` for `N`, and `unknown` for
  `B`, `R` and `E` (a bad, revoked-key or uncheckable signature is not treated as signed).
- Quoin SHALL NOT derive `status` from `statusHistory`.
- Quoin SHALL NOT take any author or date from a field typed into the document.
- If the file is not under git, then Quoin SHALL report `statusHistory` not applicable.
- If the file is not under git, then Quoin SHALL emit no transition diagnostic.
- If the working-tree `status` differs from the last entry's `status`, then Quoin SHALL append a
  pending final entry with that `status`, `commit` absent and `pending` true, and SHALL judge the
  move from the last entry to it like any other.

### Legality over squashed history

These repositories squash-merge, so several status moves can land in one commit and an
intermediate status never appears in `statusHistory`. A move from one entry to the next is
therefore legal if the later status is reachable from the earlier one through one or more legal
moves of FR-014. An
unreachable status, such as `identified` after `assessed`, or anything after `closed`, is illegal.
Because `identified` reaches every status, a first entry of any status is legal.

A reopen that lands squashed (`resolved`, `assessed`, `resolved` in one commit) leaves `status`
unchanged and is therefore not seen; `anomaly.reassessment-missing` covers only a reopen that is
visible as a separate `statusHistory` entry. This limitation is accepted rather than comparing the
assessment text across commits that leave `status` unchanged.

### Diagnostics

Each diagnostic has a `code`, a `severity`, a `reason` and the 1-based `line` of the frontmatter
`status` field in the working-tree document (a commit has no line of its own), and names the
commit or commits involved.

| Code | Severity | Reason | Condition |
|---|---|---|---|
| `anomaly.illegal-transition` | error | `transition` | The later status of a consecutive pair of `statusHistory` entries is not reachable from the earlier one. |
| `anomaly.reassessment-missing` | error | `reopen` | For a `resolved` to `assessed` move, the text of the `## Safety Impact Assessment` section is identical at the reopen commit and at the commit of the next later move out of `assessed`. |
| `anomaly.missing-module` | error | `spec-objects-safety` | The assessment holds a `Hazard` row and the `hazard` object type is not installed; one per document, reported alongside the per-row reference diagnostics of `quire validate`. |
| `anomaly.no-record` | error | `mapping-error` | The document has a mapping error and so no record. |

- Quoin SHALL emit `anomaly.illegal-transition` for each pair whose later status is unreachable.
- When a move is `resolved` to `assessed` and the assessment text did not change before the next
  later move out of `assessed`, Quoin SHALL emit `anomaly.reassessment-missing` naming the reopen
  commit.
- If the assessment holds a `Hazard` row and the `hazard` object type is not installed, then Quoin
  SHALL emit one `anomaly.missing-module` for the document.

### Reverse references

- `quoin anomaly refs <id>` SHALL list each anomaly whose `assessment` has an entry whose `target`
  resolves to `<id>`, as `{ anomaly, kind, impact }`, in anomaly id order.
- Under `--json`, `quoin anomaly refs` SHALL print the entries as one top-level array, and an empty
  array when none names `<id>`.
- Quoin SHALL include an entry whose `target` is an acceptance-criterion id.
- Quoin SHALL include no entry for a `None` row or for a `target` that resolves to nothing.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-136-AC-1 | A fixture repository whose first-parent default-branch commits set `status` to `identified`, `assessed`, `resolved`, `closed` yields four `statusHistory` entries in order, each with author, authored date, `signature` and `signatureCode`, and a file renamed along the way keeps its entries; no author or date typed in the document is read. | Test |
| FR-136-AC-2 | A document that fails only a record-schema check (a `closed` anomaly with no `closed_by`) still yields its record and `quoin anomaly check` reports no field diagnostic for it; a document with a mapping error yields `anomaly.no-record` and a `statusHistory` still checked. | Test |
| FR-136-AC-3 | Over fixture histories, `anomaly.illegal-transition` is emitted for `assessed` then `identified`, `resolved` then `identified`, `closed` then each of `identified`, `assessed` and `resolved`; and is not emitted for `identified` then `resolved` (squashed), `identified` then `closed` with `fixed` (squashed through `resolved`), `assessed` then `closed` with `fixed`, `assessed` then `closed` with `wont-fix`, `identified` then `closed` with `duplicate`, and `resolved` then `assessed`. | Test |
| FR-136-AC-4 | A history whose first entry is `resolved` or `closed` yields no diagnostic. A working tree whose `status` differs from the last default-branch entry yields a pending final entry that is judged: `closed` in history and `assessed` in the working tree yields `anomaly.illegal-transition`, and `assessed` in history and `resolved` in the working tree yields none. | Test |
| FR-136-AC-5 | A history `resolved`, `assessed`, `resolved` with the assessment section unchanged yields `anomaly.reassessment-missing` naming the reopen commit, and the same history with an edited assessment section yields none. | Test |
| FR-136-AC-6 | A fixture file outside git reports `statusHistory` not applicable and no transition diagnostic. | Test |
| FR-136-AC-7 | For a corpus with a requirement `FR-001`, its criterion `FR-001-AC-1` and a hazard, `quoin anomaly refs` on each lists the anomaly that names it with its `Impact`; on a hazard no anomaly names it prints `[]` under `--json`; an anomaly whose only row is a `None` row appears in no listing. | Test |
| FR-136-AC-8 | An anomaly with a `Requirement` row targeting `FR-999` is absent from every listing, and `quoin anomaly check` emits no dangling-reference diagnostic for it. | Test |
| FR-136-AC-9 | The anomaly documents the tests use are those shipped by `agent-ix/spec-artifacts-process` FR-014 and FR-015, including the anomaly with two `Hazard` rows and the hazard no anomaly names; the commit sequences are authored by this repository's tests, and the quoin repository contains no copy of the documents. | Inspection |
| FR-136-AC-10 | With `spec-objects-safety` not installed, an anomaly with two `Hazard` rows yields exactly one `anomaly.missing-module`, and `quire validate` still reports its own per-row diagnostics. | Test |
| FR-136-AC-11 | Every diagnostic in AC-2, AC-3, AC-5 and AC-10 carries its code, severity `error`, its reason, and the line of the document's `status` field. | Test |
| FR-136-AC-12 | Signature codes `G`, `U`, `X`, `Y` map to `signed`, `N` to `unsigned`, and `B`, `R`, `E` to `unknown`, with `signatureCode` equal to the git letter in each case. | Test |

## Dependencies

- **Upstream**: `agent-ix/spec-artifacts-process` [FR-011](ix://agent-ix/spec-artifacts-process/FR-011), [FR-014](ix://agent-ix/spec-artifacts-process/FR-014) and [FR-015](ix://agent-ix/spec-artifacts-process/FR-015) (the document shape, the record mapping, the fixtures)
- **Upstream, quire-rs**: [FR-026](ix://agent-ix/quire-rs/FR-026) (reference resolution), [FR-040](ix://agent-ix/quire-rs/FR-040) (edge target typing), and the artifact-type record extractor `agent-ix/quire-rs#393`
- **Downstream**: the anomaly views and reports that read the record
