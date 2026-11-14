---
id: SR-20261114002403
title: "Code and Rust review of PLAT-1597 audit acceptance test (PR 889)"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quoin; PR #889 at c0c39ffed4f17637f76dfdd9a0840fede3e8792c: rust/crates/quoin-cli/tests/tc_1597_criticality_mutation_floor.rs"
relationships:
  - {target: "ix://agent-ix/quoin/FR-039", type: references}
---

# Code and Rust review of PLAT-1597 audit acceptance test (PR 889)

## Summary

Ticket: PLAT-1597. This PR adds an integration test only. The rated case runs Quoin's production `evidence record` and `evidence audit` commands using `CARGO_BIN_EXE_quoin`, and resolves the actual `spec-artifacts-process/spec_artifacts_process/manifest.yaml` through `SPEC_ARTIFACTS_PROCESS_DIR`; there is no copied or vendored manifest. The control omits the authored criticality and expects no `high` floor finding. `rust/Cargo.lock` pins quire-rs 0.60.0 at `5a45ec949737a41c69a90f6ac5eefcafb2e10716`, the release with `criticality_field` support.

Review scope was the changed test, FR-039-AC-2 and FR-039-AC-7, and the locked quire-rs dependency. No production code or workflow changed, so Quoin gap-analysis is not applicable. AP-201's human acceptance gate was not run or changed; this test-only diff does not change collection, scoring, persistence, comparison, or report rendering.

The coder's supplied focused validation reports both tests passing against the real process module. My independent build attempt did not reach test execution: `quoin-semantic/build.rs` stopped because the worktree's `node_modules/@agent-ix/semantic-core/generated/json-schema` input was absent. `make build-inputs` then failed fetching `@agent-ix/semantic-core@0.4.3` from npm.ix with HTTP 401. `cargo fmt --all --check` passed and `git diff --check` passed. The independent focused test remains unverified in this checkout.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The unrated-obligation control is tagged to FR-039-AC-2, although it checks that a floor applies only to its named criticality (AC-7). This falsely attributes the test in the trace matrix. | rust/crates/quoin-cli/tests/tc_1597_criticality_mutation_floor.rs:164-180 |

## Verdict

**CONDITIONAL (one low trace finding).** The main acceptance test exercises the requested cross-boundary behavior through the real manifest, and the test itself is able to fail if a high-rated derived obligation does not yield `insufficient-mutation-score`. The lock pin and absence of a vendored manifest are correct. Change the control's trace tag to `FR-039-AC-7`; its current `FR-039-AC-2` tag claims coverage of the below-floor finding criterion, which the control does not assert. The existing auditor test for AC-7 does not make this individual test's trace claim accurate.

## Scope

- `FR-039-AC-2` (examined): "A bound score below the declared floor is reported at `medium`."
- `FR-039-AC-7` (examined): "A floor applies only to the criticality it names."
- `rust/crates/quoin-cli/tests/tc_1597_criticality_mutation_floor.rs` (examined): rated real-manifest case asserts one `insufficient-mutation-score` finding for the derived obligation; unrated control asserts that no such finding exists.
- `rust/Cargo.lock` (examined): `quire-rs` is version `0.60.0`, pinned at revision `5a45ec949737a41c69a90f6ac5eefcafb2e10716`.

## Bindings

- `tc_1597_001_real_manifest_rating_derives_floor_finding` → `FR-039-AC-2` (correct)
- `tc_1597_002_absent_rating_does_not_apply_high_floor` → `FR-039-AC-2` (incorrect; should be `FR-039-AC-7`)
