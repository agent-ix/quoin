---
id: SR-20261125092603
title: "Spec review — PLAT-1449 Quoin final head"
type: SpecReview
analysis: base
scope: "agent-ix/quoin#1069 at d294f0aa508133f31c45b68e61e960c0ed17c5c2 against origin/main"
review_set: subset
---

## Summary

FR-237 has valid links to the renumbered FR-235 and FR-236, and its mismatch and incomplete-case criteria are observable. It does not define when a verification may be recorded as successful: the only aggregate outcome rule covers incomplete or failed verification, so an implementation that always reports incomplete can satisfy the listed acceptance criteria.

## Verdict

**CONDITIONAL** — The new independent-verification requirement needs a positive completion criterion and a defined aggregate success state.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-237 requires an independent verifier but gives only a negative independence case. No criterion requires a run with distinct identified reviewer and case authors, all cases executed, and all seeded mismatches detected to be eligible for a successful/complete result. The current behavior also never defines the aggregate verification status. An implementation could mark every verification incomplete and still satisfy AC-1 through AC-5. Add an observable positive success condition and define the aggregate state. | spec/functional/FR-237-independent-validation-kit-verification.md:78-82; spec/functional/FR-237-independent-validation-kit-verification.md:20-47 |

## Coverage

Reviewed the frozen final PR head d294f0aa508133f31c45b68e61e960c0ed17c5c2 against origin/main. FR-237 relationship targets resolve to FR-235 and FR-236. FR-237 acceptance criteria are testable as stated where they cover recorded fields, missing cases, seeded mismatches, identity conflicts, and excluded judgments; the missing positive success state is the finding above. No plan was supplied. Rust behavior changes in this diff are limited to extracting run-record construction into a private helper and adding a focused serialization test; broad Cargo gates were not run. The focused Cargo test was blocked by the missing generated semantic schema under node_modules, required by quoin-semantic/build.rs. `cargo fmt --check` reports formatting drift only in two files outside this PR diff.
