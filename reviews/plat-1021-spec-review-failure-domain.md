---
id: SR-16396
title: "failure-domain review of PLAT-1021 graded-corpus collection producer"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/functional/FR-108-measurement-verdict-checker.md (FR-108-AC-15..AC-21, Graded-corpus collection producer section) -- PR #779 diff only"
review_set: subset
---

## Summary

Ticket: PLAT-1021. Failure modes of the producer and its live-harness call
site, traced against intake (`quoin-measurement` `validate/mod.rs`,
`validate/stack.rs`, `store/apparatus`) and the harness's grading types
(`tests/support/grading.rs`, `tests/support/mod.rs`). Determinism (no clock,
env or RNG; sort by `item_id`), the empty-`actual` case (counted, not matched)
and the transport-abort rule are sound. Four findings: the input cannot yield
an honest `buildProfile`, intake preconditions on the target repository are
unstated, the `actual` mapping is ambiguous for coverage rows, and the write's
position relative to the harness's earlier assertions is unstated.

## Scope examined

- FR-108-AC-15, AC-16, AC-17, AC-18, AC-19, AC-20, AC-21 (examined)
- "Where the harness calls it" paragraph (examined)

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | GradedCorpus (AC-15) carries verificationStack capabilities and artifacts only; intake refuses every new collection whose verificationStack.buildProfile is not release, and the live harness is a cargo test (debug) build, so the function must either attest a false release profile or produce a collection intake refuses; the spec must state the buildProfile source (and the stack schemaVersion) | FR-108-AC-15; FR-108-AC-21; quoin-measurement src/validate/mod.rs:231-243 |
| FND-002 | medium | AC-21 says a written collection "passes intake" but states none of intake's preconditions on the target: the measurement repository must hold MP-228 under spec/assurance, and MP-228's three protected_apparatus files must resolve there and appear in verificationStack.artifacts, or intake refuses; "the environment names a measurement repository" leaves this open | FR-108-AC-21; quoin-measurement src/store/apparatus/mod.rs:107; src/plans.rs:82 |
| FND-003 | medium | AC-21 maps fixtures to ConstantPredictorItem without saying which Graded field becomes actual; Graded carries actual (raw, "2.70" for coverage) and actual_class (rounded "3"), and the harness's agreement is verdict; mapping actual makes AC-16's matched disagree with the harness's own Tally::agreed on every non-integer coverage score, and an Unrecognized row can match by string | FR-108-AC-21; FR-108-AC-16; quoin-jev tests/support/mod.rs:420-440 |
| FND-004 | medium | The call site is "after the graded pass ... before the bars are asserted", but the gate test asserts no unanswered and no unrecognized rows before the bars; if the write follows those asserts, a pass with unanswered rows writes nothing and a failing run goes unrecorded, against AC-20's "cannot drop an item" intent and MP-228's suppressed-observation control | FR-108-AC-21; quoin-jev tests/live_criterion_strength.rs:790-815 |
