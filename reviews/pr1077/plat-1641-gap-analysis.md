---
id: SR-967175026302197768900474181941935084778
title: "Gap analysis — Quoin validation-report implementation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin repository audit: spec/, rust/crates/quoin-cli/{src,tests}, rust/crates/quoin-completeness/src"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin", type: references }
---

## Summary

This planless repository-driven audit compared the changed validation-report code and tests with the computed Test Matrix, including FR-217 and FR-220. The new report and anomaly paths have tagged tests, but FR-217-AC-8 and the external-tool report criteria FR-220-AC-2..4 remain untested; unrelated repository-wide gaps were counted separately.

## Verdict

**FAIL** — the full repository matrix contains untagged live criteria, including four criteria directly required by this delivery.

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (quoin 0.36.8; active module `spec-artifacts-process@v0.36.0`); `quire coverage --scope . --json` (quire 0.52.0, engine 0.62.2).
- Plan completion: not assessed.
- Full-repository criteria: tagged 1737; untagged 641; tagged-by-ignored-test 13; method-without-symbol 175; retired 36. Evidence status was `no run evidence` for all 2602 live criteria.
- Relevant changed/consumed criteria: FR-217-AC-8 and FR-220-AC-2..4 are untagged. FR-217-AC-1..7 and FR-220-AC-1 are tagged. FR-218-AC-4/5 bind only to the ignored real EA sign-off test, which the coder reports ran explicitly with `--ignored`.
- Changed-file coverage: no untracked test symbols; four `trace-id-outside-marker` diagnostics are module-level documentation references in `selftest.rs` and `validation/report.rs`, not test binders. No changed source or test stubs were found.
- Reverse-gap check: implementation paths inspected are owned by FR-217/FR-218/FR-220. No additional unowned runtime behavior was identified beyond the findings below.
- Semantic review: skipped; the workflow defines that pass as opt-in and no opt-in was supplied.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-217-AC-8 has no bound test and its required quire-cli/quire-rs record isolation behavior is not implemented in this Quoin-only PR. | FR-217-AC-8, rust/crates/quoin-cli/src/validation/report.rs:683 |
| FND-002 | high | FR-220-AC-2 has no bound test for resolving retained external-tool evidence to the index. | FR-220-AC-2 |
| FND-003 | high | FR-220-AC-3 has no bound test for unavailable required external tools being `not-executed` and not passing. | FR-220-AC-3 |
| FND-004 | high | FR-220-AC-4 has no bound test for missing/duplicate index entries or run/index identity mismatches. | FR-220-AC-4 |
## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | FR-217-AC-8 is Quire-side work tracked by PLAT-1642, which remains open; this replacement PR title and completion scope are PLAT-1641 only. |
| FND-002 | fixed | Verified on PR head 79eaa3e8. |
| FND-003 | fixed | Verified on PR head 79eaa3e8. |
| FND-004 | fixed | Verified on PR head 79eaa3e8. |

+++ [reviewer dispositions]

```yaml
dispositions:
- fnd: FND-001
  outcome: deferred
  reason: FR-217-AC-8 is Quire-side work tracked by PLAT-1642, which remains open;
    this replacement PR title and completion scope are PLAT-1641 only.
- fnd: FND-002
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-2")]

    #[test]

    fn external_tool_evidence_resolves_exactly_and_preserves_observed_version() {'
- fnd: FND-003
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-3")]

    #[test]

    fn unavailable_external_tool_case_remains_not_executed_and_not_passing() {'
- fnd: FND-004
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-4")]

    #[test]

    fn invalid_index_entries_and_mismatched_evidence_do_not_hide_valid_entries() {'
```

+++

## New findings (replacement PR #1077 review)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | FR-236-AC-8 has no computed-matrix binder for the run-record anomaly-set serialization behavior introduced by this PR. | FR-236-AC-8 |

The current matrix command was `quoin matrix --repo . --json`; FR-236-AC-8 reports `static_status: untagged`, `binders: []`, and “no evidence is bound to FR-236-AC-8”. The existing `tc_230_run_record_serializes_run_context_and_case_outcomes` test is tagged to FR-236-AC-1, AC-2, and AC-4, not AC-8; its current assertions do not cover all AC-8 behavior.
