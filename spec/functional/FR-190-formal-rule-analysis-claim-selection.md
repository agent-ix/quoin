---
id: FR-190
title: "Formal-rule analysis claim selection excludes drafts"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "depends_on"
  - target: "ix://agent-ix/filament-analyze/FR-009"
    type: "references"
---

# FR-190: Formal-rule analysis claim selection excludes drafts

## Description

Quoin SHALL emit, from the [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md)
report, a `filament-analyze.claim-selection/v1` document that lists the source
spans of `approved` declarations only, so that a `draft` rule never judges an
output. The document is the one that `filament-analyze run-package --select`
reads, and its "Claim selection" section in
[filament-analyze FR-009](ix://agent-ix/filament-analyze/FR-009) is its sole
definition; this FR does not restate the schema. Origin: VER-98.

## Inputs

The inputs of `quoin formal-rules status` (FR-188), plus `--select-out <abs path>`.

## Outputs

`quoin formal-rules status --select-out <path>` writes to `<path>` one JSON
document:

```json
{"protocol":"filament-analyze.claim-selection/v1","spans":[{"authority":"quoin","identity":"rules/a.qsl","start":120,"end":340}]}
```

- `spans` holds one entry per `approved` declaration, built from that
  declaration's FR-188 report entry.
- `authority` is the string `quoin` and `identity` is the report entry's
  repo-relative `path`, byte for byte, with no normalization. Together they are
  the source reference (`authority`, `identity`) of the region that the
  package's source map records for the declaration's file. The source map's
  contract is owned by filament-analyze FR-009 ("Boundary with the producer of
  `analysis_claim` nodes"): the package is produced from the same source
  identity and bytes as the HEAD content from which `declaration_range` is
  computed. A package produced from other content is outside this FR.
- `start` and `end` are the entry's `declaration_range`: a half-open interval of
  UTF-8 byte offsets, `start` inclusive and `end` exclusive, with `start` below
  `end`. Spans never use line numbers.
- `spans` is sorted by `identity`, then `start`.
- With no `approved` declaration, `spans` is the empty list `[]`. The document
  is then well formed (filament-analyze FR-009) and is written: it states that no
  formal rule is approved. It is not a usage error, and the command exits as
  FR-188 does. The approval workflow SHALL NOT invoke `run-package` for it,
  because the runner refuses an empty selection with exit `2` and the cause
  `empty selection`; no analysis runs and no report is produced.

Exit status is that of FR-188.

## Behavior

- A `draft` declaration SHALL NOT appear in the document, whatever its reason.
- Quoin SHALL write no other state, and no hash, for the selection.
- The consumer (`run-package --select`, filament-analyze FR-009) owns the claim
  matching and the report `selection` member; this FR defines neither.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-190-AC-1 | Given a fixture with one `approved` and one `draft` declaration, the document lists only the approved declaration, with `authority` `quoin`, `identity` its repo-relative path, and `start`/`end` equal to its FR-188 `declaration_range` | Test |
| FR-190-AC-2 | Given a fixture with no `approved` declaration, the document is `{"protocol":"filament-analyze.claim-selection/v1","spans":[]}` and the command exits as FR-188 does | Test |
| FR-190-AC-3 | Given a fixture where an approved declaration is edited after approval, its span is absent from the document | Test |
| FR-190-AC-4 | Given an approved declaration that follows a multi-byte character in its file, slicing the file bytes at `[start, end)` yields the declaration span, and no member of the document is a line number | Test |
| FR-190-AC-5 | Given approved declarations in two files, `spans` is ordered by `identity`, then `start` | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md);
  [filament-analyze FR-009](ix://agent-ix/filament-analyze/FR-009)
  (`run-package --select`, consumer and owner of the selection schema).
