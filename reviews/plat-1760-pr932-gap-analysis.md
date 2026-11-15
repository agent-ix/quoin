---
id: SR-20261115210123
title: "Gap analysis — PLAT-1760 PR #932 typed stale-evidence cause"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-032-evidence-auditor.md, spec/functional/FR-115-evidence-backed-test-matrix.md, spec/functional/FR-167-release-snapshot-format.md, rust/crates/quoin-auditor, rust/crates/quoin-assurance, rust/crates/quoin-core/src/ops/matrix.rs, rust/crates/quoin-cli/src/release/snapshot.rs; PLAT-1760; PR #932 head 2c7d90d3"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-032"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-167"
    type: "references"
---

## Summary

Ticket: PLAT-1760. This is a planless gap analysis of the criteria PLAT-1760
adds: FR-032-AC-18..21, FR-115-AC-17..25 and FR-167-AC-10..12.

`quoin matrix` and `quire matrix` could not be run, because the sandbox
requires approval for both binaries. The matrix step therefore used the grep
fallback: `#[trace("…")]` tags under `rust/` for each criterion id.

No tagged test has run. The PR does not compile (see SR-20261115210122
FND-001), so every binding below is a tag, not evidence.

## Verdict

**FAIL.** Coverage of the 16 criteria:

- **Untagged (4):** FR-115-AC-20 and FR-167-AC-10..12.
- **Inflated (1):** FR-115-AC-22's only test is an empty placeholder.
- **Partially asserted (2):** FR-032-AC-21 and FR-115-AC-23 each have a test
  that checks only part of the criterion.
- **Tagged and asserting their criterion (9):** FR-032-AC-18, AC-19 and AC-20,
  and FR-115-AC-17, AC-18, AC-19, AC-21, AC-24 and AC-25. Among these, the
  AC-20 test `tc_1760_deterministic_stale_suite_selection_failed_first` is
  correct against the spec. It is the code that disagrees (SR-20261115210122
  FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `tc_1760_invalid_cause_on_stale_refused` carries `#[trace("FR-115-AC-22")]` but has an empty body that asserts nothing. This is coverage inflation: the matrix will show AC-22 as tagged when nothing verifies it. | rust/crates/quoin-assurance/tests/tc_1760_matrix_cause.rs:170-186 |
| FND-002 | medium | FR-115-AC-20 is untagged. No test covers a passing but uninspected suite (`undischarged` through `unevaluated`) having no stale finding and no `cause`. | rust/crates/quoin-assurance/tests/tc_1760_matrix_cause.rs |
| FND-003 | medium | FR-167-AC-10 is untagged. No snapshot test checks that `open_findings` carries the `missing-run` and `failed-run` causes byte for byte. | rust/crates/quoin-cli/src/release/snapshot.rs:208-216 |
| FND-004 | medium | FR-167-AC-11 is untagged. No snapshot test checks that a non-stale `open_findings` item has no `cause` key. | rust/crates/quoin-cli/src/release/snapshot.rs:95-100 |
| FND-005 | medium | FR-167-AC-12 is untagged. No snapshot test checks that a stale finding without `cause` serializes with no key, rather than `null`. | rust/crates/quoin-cli/src/release/snapshot.rs:95-100 |
| FND-006 | medium | FR-115-AC-23 requires `Refused` with `context.reason` `invalid-stale-cause` naming the obligation. Its test asserts only the library `MatrixRefusal::InvalidStaleCause` variant. The `quoin-core` mapping to `CoreErrorCode::Refused` and its context is untested. | rust/crates/quoin-assurance/tests/tc_1760_matrix_cause.rs:191-220 |
| FND-007 | medium | FR-032-AC-21's test checks only that an `undischarged` finding has no `cause` key. The criterion's second clause, that `cause` changes no `summary`, severity, finding order or healthy set, is not asserted. | rust/crates/quoin-auditor/tests/tc_1760_stale_cause.rs:231-255 |

## Coverage

- Examined criteria: FR-032-AC-18, AC-19, AC-20 and AC-21; FR-115-AC-17
  through AC-25; FR-167-AC-10, AC-11 and AC-12.
- Read as context: FR-032-AC-17, plus the FR-032 Behavior, FR-115 "Typed
  cause" and FR-167 ordering clauses.
- Matrix source: grep fallback. `quoin matrix` was unavailable because
  approval was required.
- Reverse gap: none. Every new production line traces to FR-032, FR-115 or
  FR-167.
- Stubs: the comment-only stale-evidence branch in `matrix.rs:432-436`
  (recorded as SR-20261115210122 FND-003), and the placeholder test (FND-001
  here).
- Semantic review: skipped (not opted in).
- Plan completion: not assessed


## New findings (final disposition)

The round-2 Opus disposition at head `467c317c` added FND-008; the final permitted fix round addressed it.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | FR-115-AC-22/23 had no test of the operation-to-core mapping returning Refused with `context.reason=invalid-stale-cause` and the obligation id. | rust/crates/quoin-core/tests/tc_1080_matrix_boundary.rs |

## Dispositions

Latest outcomes at PR #932 head `104078c6`, after fix round 3. Focused tests and Clippy passed for quoin-graph-analysis, quoin-auditor, quoin-assurance and quoin-finding-types. The final aggregate gate remains pending.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The invalid-cause test now asserts refusal at the matrix operation with the offending obligation, rather than a deserialization error. |
| FND-002 | fixed | The uninspected suite test uses the actual `Undischarged` status and verifies no stale finding is added. |
| FND-003 | fixed | The snapshot fixture contains one failed-run and one missing-run suite and asserts both causes in the exact open-findings list. |
| FND-004 | fixed | The snapshot test asserts non-stale findings have no cause key. |
| FND-005 | fixed | A snapshot unit test feeds a stale finding without a cause through `sections()` and asserts the serialized cause key is absent, not null; it also checks a present cause is retained. |
| FND-006 | fixed | Quoin-core boundary tests assert exit 2, `CORE_REFUSED`, `context.reason=invalid-stale-cause`, and the obligation for invalid stale and non-stale causes. |
| FND-007 | fixed | The auditor test compares reports with and without causes and pins summaries, severity, finding order, and healthy set. |
| FND-008 | fixed | The quoin-core test covers the public Refused code and context reason required by FR-115-AC-22/23. |
