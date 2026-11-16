---
id: SR-20261116075204
title: "Gap analysis — PLAT-1760 PR #945 release snapshot fixture"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/; rust/crates/quoin-cli/src/release/snapshot.rs; rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs; PLAT-1760; PR #945"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-032", type: references }
  - { target: "ix://agent-ix/quoin/FR-115", type: references }
  - { target: "ix://agent-ix/quoin/FR-167", type: references }
---

## Summary

Ran a planless matrix and gap review for the release-snapshot cause test. The changed fixture contains a real behavioral assertion and introduces no source behavior, but the computed matrix reports FR-167-AC-10 as untagged on both this branch and clean `origin/main` even though the test carries the declared Rust trace attribute.

## Verdict

**FAIL** — FR-167-AC-10 is untagged in the computed matrix; the same result reproduces on clean `origin/main` and predates PR #945.

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies because this review inspects the release snapshot's rendering of typed evidence findings. Its review policy is `recommend`; this run is requested review work, not a human approval gate. Matrix and coverage outputs are the available evidence; no controlled-corpus adjudication, selected measurement plan, architecture evidence, independence evidence, producer-reliance record, or active exception is in scope for this test-only PR. AP-202 does not apply.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix reports FR-167-AC-10 as untagged and lists no binder for `tc_1760_snapshot_carries_stale_causes_byte_for_byte`, despite its existing `#[trace("FR-167-AC-10")]` marker. The same result occurs on clean `origin/main`; PR #945 does not change the marker. | FR-167-AC-10; rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs::tc_1760_snapshot_carries_stale_causes_byte_for_byte |

## Coverage

- Reconciliation: `quoin matrix --repo <repo> --json` (Quoin 0.35.0) and `quire matrix --scope <repo> --format json` (Quire 0.45.0, engine 0.60.0); no grep fallback. Both tools report 18 tagged, 1,958 untagged, 143 method-without-symbol, and 37 retired criteria on the PR and clean `origin/main`.
- FR-167-AC-10 is reported `untagged` on both refs. Quoin also reports its evidence status as `no run evidence`; the same tag grammar diagnostics appear in `quire coverage`.
- `quire coverage --scope <repo> --json` reports 289 untracked symbols and 3,736 diagnostics on the PR worktree. These are repository-wide baseline outputs and were not introduced by this test-only diff.
- Reverse gap for the PR diff: no production behavior was added or changed. The snapshot output remains owned by FR-167 and its typed source by FR-115/FR-032. The changed test has a real CLI invocation and exact-value assertion; no test stub was found in the diff.
- Plan completion: not assessed. Semantic review: skipped; not requested.
- Baseline comparison: Quoin and Quire matrix counts match clean `origin/main`; FR-167-AC-10 remains untagged there, so this is a verified baseline issue rather than a PR-introduced regression.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The untagged matrix result reproduces on clean origin/main; PR #945 does not change the marker or trace grammar, and the lead has recorded this baseline limitation in the PR under the moving-main rule. |
