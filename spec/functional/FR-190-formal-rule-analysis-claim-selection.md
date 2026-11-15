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

When `quoin formal-rules status` is given `--select-out <path>`, Quoin SHALL write
to `<path>` a claim-selection document naming the declaration spans of the
[FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md) report's
`approved` declarations only, so that a `draft` rule never judges an output.
The document schema, its protocol name, the span members, the interval
semantics and how an empty selection is handled are owned by the "Claim
selection" section of [filament-analyze FR-009](ix://agent-ix/filament-analyze/FR-009)
and are not restated here. This FR defines only how Quoin maps the FR-188 report
onto that document. Origin: VER-98.

## Inputs

The inputs of `quoin formal-rules status` (FR-188), plus `--select-out <abs path>`.

## Outputs

- `spans` holds one span per `approved` declaration, built from that declaration's
  FR-188 report entry.
- Each span's `start` and `end` are the entry's `declaration_range`; FR-188 owns
  that interval and its validity.
- Precondition: the contract package that the selection is applied to is produced
  from the same source identity and the same bytes of each file as the HEAD
  content from which `declaration_range` was computed. For a package produced
  from other content, the spans carry no meaning and this FR makes no claim.
- Each span's `identity` is the entry's repo-relative `path`, byte for byte, with
  no normalization. The spelling of the `authority` and of the source reference
  that a package's source map records for a QSL file is owned by the producer of
  the package's `analysis_claim` nodes (Linear ticket QSL-1900)
  and is not defined here. This is an unresolved upstream dependency; this FR
  does not fix the `authority` value.
- `spans` is ordered by `identity` in UTF-8 byte order, then by `start`.
- With no `approved` declaration, `spans` is the empty list. Quoin writes that
  document and exits as FR-188 does; it is not a usage error. What the approval
  workflow does with it is specified by [FR-189](./FR-189-formalize-requirement-workflow.md).

Exit status is that of FR-188, except for an invalid `--select-out`.

## Behavior

- A `draft` declaration SHALL NOT appear in the document, whatever its reason.
- Quoin SHALL write no other state, and no hash, for the selection.
- If `<path>` is relative, then Quoin SHALL exit `2` before reading the repository
  and write nothing.
- If FR-188 exits `2` or `3`, then Quoin SHALL leave no file at `<path>`, removing
  any file already there, so that a failed run never leaves a selection from an
  earlier run.
- If `<path>` cannot be written, then Quoin SHALL exit `2` and leave no file at
  `<path>`.
- Quoin SHALL write the document completely or not at all.
- The consumer (`run-package --select`, filament-analyze FR-009) owns the claim
  matching and the report `selection` member; this FR defines neither.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-190-AC-1 | Given a fixture with one `approved` and one `draft` declaration, the document lists only the approved declaration, with `identity` its repo-relative path and `start`/`end` equal to its FR-188 `declaration_range` | Test |
| FR-190-AC-2 | Given a fixture with no `approved` declaration, the document holds an empty `spans` list and the command exits as FR-188 does | Test |
| FR-190-AC-3 | Given a fixture where an approved declaration is edited after approval, its span is absent from the document | Test |
| FR-190-AC-4 | Given an approved declaration that follows a multi-byte character in its file, slicing the file bytes at `[start, end)` yields the declaration span | Test |
| FR-190-AC-5 | Given approved declarations in files `rules/b.qsl`, `rules/z.qsl` and `rules/é.qsl`, `spans` lists them in that order, the UTF-8 byte order of `identity` | Test |
| FR-190-AC-6 | Given approved declarations at two offsets of one file, `spans` lists them by ascending `start` | Test |
| FR-190-AC-7 | Given a relative `--select-out`, the command exits `2` and writes no file | Test |
| FR-190-AC-8 | Given an existing file at `--select-out` and a run where FR-188 exits `2`, no file remains at that path | Test |
| FR-190-AC-9 | Given an existing file at `--select-out` and a run where FR-188 exits `3`, no file remains at that path | Test |
| FR-190-AC-10 | Given a `--select-out` in a directory that does not exist, the command exits `2` and writes no file | Test |
| FR-190-AC-11 | The document of AC-1 is accepted by `filament-analyze run-package --select` as a well-formed selection | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-formal-rule-approval-state-from-signed-commits.md);
  [filament-analyze FR-009](ix://agent-ix/filament-analyze/FR-009)
  (consumer and owner of the selection schema); QSL-1900 (owner of the
  source-map source-reference spelling, not yet specified).
