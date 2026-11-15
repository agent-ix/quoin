---
id: SR-20261115183518
title: "Gap analysis — PLAT-1760 FR-115-AC-17..19 typed stale-evidence cause"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md, rust/crates/quoin-finding-types, rust/crates/quoin-auditor, rust/crates/quoin-assurance, rust/crates/quoin-cli/tests; PLAT-1760"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-115"
    type: "references"
---

## Summary

Ticket: PLAT-1760. Planless gap analysis scoped to the three criteria PR #931
adds, FR-115-AC-17..19. `quoin matrix` and `quire matrix` could not be run
because the sandbox requires approval for both binaries. The matrix step
therefore used the grep fallback: every `FR-115-AC-*` trace tag under `rust/`.
This PR changes the spec only, so code lagging the spec is expected. These
findings record that state accurately and do not claim a regression.

## Verdict

**FAIL**, by the skill's mechanical rule: all three new criteria are
untagged. This is the expected state for a spec-ahead PR, not a defect in the
PR. Implementation is the follow-up. AC-1..AC-16 keep their existing tags,
1 to 5 each, and nothing they cover changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-115-AC-17 is untagged, and `cause: missing-run` is not implemented. `quoin_finding_types::Finding` has no `cause` field, and `unrecorded_evidence` mints a `stale-evidence` finding with only kind, obligation, severity and summary. The nearest test is `tc_1080_matrix_command.rs:341`, which reaches the missing-run case through its summary only, and it is not tagged AC-17. | rust/crates/quoin-auditor/src/audit/stale.rs:39 |
| FND-002 | medium | FR-115-AC-18 is untagged, and `cause: failed-run` is not implemented: `failed_run` sets no cause. | rust/crates/quoin-auditor/src/audit/stale.rs:85 |
| FND-003 | medium | FR-115-AC-19 is untagged. `FindingDetail` in `matrix.rs` reduces each finding to kind, summary, path, line and symbol, with `deny_unknown_fields`, so no `cause` reaches `evidence_detail.findings` yet. | rust/crates/quoin-assurance/src/matrix.rs:251 |

## Coverage

- Criteria examined: FR-115-AC-17, FR-115-AC-18 and FR-115-AC-19, all untagged.
  FR-115-AC-4, AC-6 and AC-9 were read as context.
- Matrix source: grep fallback. `quoin matrix` was unavailable because
  approval was required.
- Reverse gap: none. No code exists for `cause`.
- Stubs: none found in the producers or the matrix reduction.
- Semantic review: skipped (not opted in).
- Plan completion: not assessed

## Dispositions

Round 1, fix commit 0883d0c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Implementation and tests for the missing-run producer are in the PLAT-1760 implementation PR, outside this spec-only PR. |
| FND-002 | deferred | Implementation and tests for the failed-run producer are in the PLAT-1760 implementation PR, outside this spec-only PR. |
| FND-003 | deferred | Matrix passthrough implementation and tests are in the PLAT-1760 implementation PR, outside this spec-only PR. |
