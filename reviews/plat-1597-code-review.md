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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The unrated control now carries `#[trace("FR-039-AC-7")]`, matching its assertion that a criticality-specific high floor does not apply when the obligation has no rating. |

### PR #900 review (disposition pass 2)

The frozen PR #900 head changes only the synthetic verification method to the catalog id `mutation-testing` and records an empty mock inspection in the test fixture. It consumes the current `spec-artifacts-process` main manifest at `2ca18d8647e09a0bd61c114226b0d6971555f7eb`, where acceptance-criterion sources declare `criticality_field: criticality`; the manifest remains external and unmodified. The rated derived obligation still asserts exactly one matching `insufficient-mutation-score` finding, and the absent-rating control still asserts none. No new finding was identified. Both focused tests passed; `cargo fmt --all --check`, clippy for `quoin-cli`, and `cargo deny check` passed. The full crate suite did not complete in this sandbox because unrelated `tc_1515_soup_promote` tests require the missing `spec-objects-security` checkout (`SPEC_OBJECTS_SECURITY_DIR`).

### PR #900 review (disposition pass 3)

Reviewed PR #900 head `56f89b3587c299c377894d962a4ab2e02bef2c25` against fetched `origin/main` `b8eac0189e358ffa2d78cd0bfce2f624fa9180d9`. The current diff adds authored `Decision`, `Decision owner`, `Approval ref`, and optional expiry mapping for scope exclusions. The field mapping agrees with spec-objects-security FR-016 and Quoin FR-173; `Responsible` remains required as a boundary assertion and is not used as the fact owner. The current external spec-objects-security main at `452f3686985f0d3fb994558659d4ecfd39ac7cf1` requires the exact authored fields and separates `decision_owner` from `responsible`.

Two new findings remain open. First, the AC-4 test only exercises `delegated`, not `accepted_risk` or `temporary_exception`, owner and approval preservation for those cases, or expiry copying/omission. Second, the new expiry assignment uses mutable JSON indexing, which violates the workspace `clippy::indexing_slicing` lint and makes the required clippy command fail. `tc_173_facts_from_mappings` passed 16/16; `cargo fmt --all --check` passed; `cargo deny check` passed with the existing yanked `yoke-derive 0.8.3` warning. Full `cargo test -p quoin-cli` is still running.

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The FR-173-AC-4 test exercises only `delegated`, leaving the authored `accepted_risk` and `temporary_exception` mapping and expiry behavior untested. | rust/crates/quoin-cli/tests/tc_173_facts_from_mappings.rs:515-529 |
| FND-003 | medium | The new expiry field assignment uses forbidden `clippy::indexing_slicing`; the required `quoin-cli` clippy gate fails. | rust/crates/quoin-cli/src/evidence/mapping_facts.rs:421 |

### FND-002 details

- Requirement: FR-173-AC-4 — “A binding clause with each of `delegated`, `accepted_risk` and `temporary_exception` authored on its `scope_exclusion` yields a disposition fact with that exact decision, the authored `decision_owner` mapped to fact `owner`, and the approval reference in separate fields; for `temporary_exception`, authored `expires_at` equals fact `decisionExpiresAt`, while the other decisions omit that field. No value is inferred from the boundary assertion, responsible party or command options.”
- The test helper always authors only `delegated`; the assertion checks that one decision, owner, approvalRef and rationale. A regression mapping `accepted_risk` incorrectly or omitting `decisionExpiresAt` for temporary exceptions would leave the only AC-4 test green.

### FND-003 details

- The changed line is `fact["decisionExpiresAt"] = json!(expires_at);` in production code. `cargo clippy --locked -p quoin-cli --all-targets --all-features -- -D warnings` fails at this line because indexing may panic and the repo forbids `clippy::indexing_slicing` in production.

## Bindings (pass 3)

- `tc_173_a_scope_exclusion_yields_its_authored_disposition` → `FR-173-AC-4` (tag is relevant; assertion covers only the `delegated` branch, as described in FND-002).

### Dispositions (pass 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | The current test only constructs and asserts `delegated`; `accepted_risk`, `temporary_exception`, expiry copying and expiry omission remain unpinned. |
| FND-003 | still-open | The required clippy command fails at the new JSON indexing expression with `clippy::indexing_slicing`. |

## New finding (contract integration, pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The AC-4 fixture uses a synthetic schema that bypasses the current security module contract, whose manifest still extracts only Set, Clause, Responsible and Rationale; the new disposition fields and decision-owner resolution are not contract-validated. | rust/crates/quoin-cli/tests/tc_173_facts_from_mappings.rs:311-329; ../spec-objects-security-main/spec_objects_security/manifest.yaml:650-674 |

### FND-004 details

The `scope_exclusion` test module defines `data_schema: {type: object}` and extracts only id/title/type. The latest `spec-objects-security` main manifest (`091b4f627196cf1859f6e7a20efff5bff7678f0d`) still maps only `set`, `clause`, `responsible`, and `rationale` (manifest lines 650-674); its `ScopeExclusion.json` requires only those four keys and rejects unevaluated properties (lines 5-31). Its FR-016 prose now requires `decision`, `decision_owner`, `approval_ref`, and conditional `expires_at`, and requires `decision_owner` to name a same-bundle person or authority artifact. Because the test installs the permissive synthetic module, it cannot show that an actual authored scope_exclusion is validated against the contract before Quoin reads the new raw sections. The code currently emits a fact when the decision owner is a valid-looking SemanticId that does not resolve to a bundle artifact. Align the module's manifest/schema with FR-016 and exercise this path against that contract, or otherwise enforce the contract in the command.

### Dispositions (pass 3, continued)

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | still-open | The only changed mapping test uses a permissive synthetic `scope_exclusion` schema. Latest security-module main still omits disposition fields from extraction/schema and the mapping path does not resolve the decision owner to a bundle artifact. |

### Final-head confirmation for PR #900 (disposition pass 4)

Final frozen PR head `16fc091d179e2b025f697acdc9ab2027f650e353` was fetched and reviewed against current `origin/main` `b8eac0189e358ffa2d78cd0bfce2f624fa9180d9`. The net diff is one file: `rust/crates/quoin-cli/tests/tc_1597_criticality_mutation_floor.rs`. It changes the fixture's verification method from the non-catalog label to the catalog id `mutation-testing` and records an empty mock inspection before audit. It does not vendor or modify the current `spec-artifacts-process` manifest. The rated derived obligation still asserts exactly one `insufficient-mutation-score` finding; the unrated control still asserts none. No new finding applies to this final diff. The coder's earlier focused locked-build result reports both tests passed; no builds were rerun for this final-head-only confirmation.

The PLAT-1525 facts-from-mappings changes reviewed in pass 3 have been removed from this PR and routed to the separate PLAT-1525 follow-up. This pass supersedes the pass-3 `still-open` outcomes with `deferred` dispositions; the earlier evidence remains unchanged.

### Dispositions (pass 4)

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | deferred | The FR-173 facts-from-mappings code and tests are absent from the final PLAT-1597 diff and are being handled under PLAT-1525. |
| FND-003 | deferred | The clippy violation was in the removed PLAT-1525 facts-from-mappings code, not in the final PLAT-1597 diff. |
| FND-004 | deferred | The scope-exclusion contract integration issue belongs to the removed PLAT-1525 facts-from-mappings work and is routed to that follow-up. |
