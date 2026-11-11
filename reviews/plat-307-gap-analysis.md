---
id: SR-20261111200816
title: "Gap analysis of quoin PR 814 (FR-135 requirements mapping matrix)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@6ed10483; spec/functional/FR-135-requirements-mapping-matrix-view.md, spec/matrix.md, rust/crates/quoin-assurance/src/discharge/rmm.rs, rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs, rust/crates/quoin-cli/tests/tc_3206_requirements_matrix_command.rs"
review_set: subset
---

# Gap analysis of quoin PR 814

## Summary

Ticket: PLAT-307. Planless. Plan completion: not assessed. `quoin matrix --json`
(quoin 0.30.0) lists FR-135-AC-1..AC-7 as `tagged`, with binders at the rmm.rs unit
tests, tc_3205 and tc_3206. It reports no `criterion-tagged-outside-row` or
`trace-id-outside-marker` for FR-135. The ids are free: FR-135 is unused on origin/main
(f4ffc262; #811 dropped FR-134 before merge, so FR-134 is unused too). TC-3200..3206 do
not collide with the open PRs #812 (TC-2900/2901), #813 (TC-3100..3102) or #815
(TC-3000..3005, no new FR). The matrix.md FR row, criteria row and seven TC rows are
present. Every AC has a test that asserts its main claim. The gaps are adverse cases the
spec and the coordinator's brief require that no test exercises.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test proves a stored disposition with an empty or whitespace-only rationale refuses the matrix (no rationale-less Tailored row can appear). The refusal does propagate (record: `DischargeFact::try_from`; read: `discharge_report_from_store` re-parses through `parse_fact`, which gives DischargeError, CORE_BAD_REQUEST, exit 3), but nothing in the FR-135 suite holds it. | rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs:787-855; rust/crates/quoin-assurance/src/discharge/store.rs:383-401 |
| FND-002 | low | TC-3202 does not cover the not-yet-valid case (attestedAt after asOf) that FR-135 names, and does not assert `tailoring_rationale` is None for the expired disposition. | rust/crates/quoin-assurance/src/discharge/rmm.rs:409-440 |
| FND-003 | low | TC-3204's determinism check renders one struct twice. A pure function agrees with itself whatever it does. Determinism should be shown across two independent builds (facts supplied in different orders, or two store reads), compared against a literal expected row. | rust/crates/quoin-assurance/src/discharge/rmm.rs:461-471 |
| FND-004 | low | TC-3206 asserts only `!status.success()` for `--rmm` without `--repo` and for `--rmm --inward`. A crash or an unrelated error passes. Assert the exit status and the refusal message. | rust/crates/quoin-cli/tests/tc_3206_requirements_matrix_command.rs:242-251 |

## Verdict

**MERGEABLE AFTER FND-001 IS FIXED.** The test for FND-001 should hand-write a stored record
under the clause-set directory whose disposition has `"rationale": " "`, the way the
tc_2602 store tests write raw files. It should then call `evidence.requirements_matrix`
and assert exit 3, empty stdout and a rationale message. Add a unit case in rmm.rs as
well: `build_discharge_report` errs on a rationale-less disposition, so no matrix
exists. Tag the tests to the AC that spec-review FND-004 adds. FND-002 to FND-004 are
cheap and belong in the same fix round.
