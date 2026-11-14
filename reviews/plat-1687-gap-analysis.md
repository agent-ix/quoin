---
id: SR-20261113203418
title: "Gap analysis of PLAT-1687 Test Result export"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1687 PR #888: FR-185 and its implementation/tests in quoin-evidence and quoin-cli"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-185"
    type: references
---

## Summary

Planless, PR-scoped traceability check for FR-185, its tagged tests, and the export implementation. `quire matrix` finds test binders for AC-1 through AC-6 and AC-8, and an inspection-test binder for AC-7; the production mapping function names FR-185, but the CLI adapter does not carry the implementation tag required by the repository's Rust convention. Plan completion: not assessed.

## Verdict

CONDITIONAL — one production route is missing its `#[implements]` trace link.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new CLI `run` adapter implements FR-185's argument, store, exit-code, and output behavior but lacks `#[implements("FR-185")]`; the tag on `build_statement` covers only the pure mapping function. | rust/crates/quoin-cli/src/evidence/test_result.rs:34-37; rust/crates/quoin-evidence/src/export/test_result.rs:112 | correct-requirement-no-evidence |

## Coverage

`quire matrix --scope /home/peter/dev/quoin/.worktrees/plat-1687 --format json` exited 0. It reports FR-185-AC-1 through AC-6 and AC-8 as tagged, with the listed test functions in both CLI and evidence integration tests where applicable. AC-7 has a tagged static-inspection test binder and the matrix classifies its `Inspection` verification method as `method-without-symbol`.

The separate installed `quoin matrix --repo /home/peter/dev/quoin/.worktrees/plat-1687 --json` also exited 0, but reports the Rust trace-tagged criteria as untagged throughout this Quoin checkout, including pre-existing FR-184 criteria. I did not treat that existing tool discrepancy as a PR defect; Quire's static matrix provides the source-level binder result for this review.

Reverse-gap review found one attributable omission: `quoin-cli/src/evidence/test_result.rs::run` has no `#[implements]` annotation. `quoin-evidence::export::test_result::build_statement` carries `#[implements("FR-185")]`. The route test suite covers CLI behavior, the evidence test suite covers the pure mapping, and neither changes code outside FR-185's scope. No stubs or coverage inflation were found in the reviewed path.

Plan completion: not assessed.


## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |

- FND-001 after_excerpt (`rust/crates/quoin-cli/src/evidence/test_result.rs:35-36`): `#[implements("FR-185")]` is on the CLI `run` adapter.
