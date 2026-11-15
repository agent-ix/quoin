---
id: FR-190
title: "Formal-rule analysis claim selection excludes drafts"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "depends_on"
---

# FR-190: Formal-rule analysis claim selection excludes drafts

## Description

Quoin SHALL emit, from the [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md)
report, a `quire-analyze.claim-selection/v1` document that lists the source
spans of `approved` declarations only, so that a `draft` rule never judges an
output. Origin: VER-98.

## Inputs

The inputs of `quoin formal-rules status` (FR-188), plus `--select-out <abs path>`.

## Outputs

`quoin formal-rules status --select-out <path>` writes the document to `<path>`
with one entry `{path, start_line, end_line}` per `approved` declaration, using
the declaration span the `qsl` adapter reports at HEAD, sorted by `path`, then
`start_line`. With no `approved` declaration it writes an empty list. Exit
status is that of FR-188.

## Behavior

- A `draft` declaration SHALL NOT appear in the document, whatever its reason.
- The consumer (`quire-analyze run-package --select`, filament-analyze) owns the
  claim matching and the `selection` report member; this FR defines neither.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-190-AC-1 | Given a fixture with one `approved` and one `draft` declaration, the document lists only the approved declaration's span | Test |
| FR-190-AC-2 | Given a fixture with no `approved` declaration, the document holds an empty list | Test |
| FR-190-AC-3 | Given a fixture where an approved declaration is edited after approval, its span is absent from the document | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md);
  filament-analyze `run-package --select` (consumer).
