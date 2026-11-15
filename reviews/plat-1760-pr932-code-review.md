---
id: SR-20261115210122
title: "Code review (Rust lane) — PLAT-1760 PR #932 typed stale-evidence cause"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-finding-types/src/finding.rs, rust/crates/quoin-finding-types/src/lib.rs, rust/crates/quoin-auditor/src/audit/stale.rs, rust/crates/quoin-assurance/src/matrix.rs, rust/crates/quoin-core/src/ops/matrix.rs, rust/crates/quoin-cli/src/release/snapshot.rs, rust/crates/quoin-auditor/tests/tc_1760_stale_cause.rs, rust/crates/quoin-assurance/tests/tc_1760_matrix_cause.rs; PLAT-1760; PR #932 head 2c7d90d3"
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

Ticket: PLAT-1760. PR #932 (head `2c7d90d3`) adds a `StaleCause` enum and a
`cause` field on `Finding`, sets it in the auditor, and passes it through
`matrix.build` and the release snapshot. This review covers the full diff
against `origin/main`. The `rust-review` lane is folded in, under the repo's
`.claude/skills/rust-style` and the workspace lint policy.

Measured: `cargo check -p quoin-finding-types --locked` fails with
`unexpected closing delimiter` at `finding.rs:163`. No aggregate build, lint
or test gate was run, as the brief asked. Every other finding was traced by
reading the code against FR-032, FR-115 and FR-167.

## Verdict

**FAIL.** The PR does not compile, so none of its 16 new tests has ever run.
Three behaviours contradict the spec even once it compiles:

- AC-20 suite selection is wrong because of the ladder order.
- AC-22 cannot reach `invalid-stale-cause`.
- The snapshot sorts an absent cause last, where FR-167 says first.

What is right: the field is `Option` with `skip_serializing_if` (no `null` on
the wire), the core refusal maps to `Refused` with `context.reason`, and
cause-on-non-stale is refused.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A stray closing brace after `impl Finding` means `quoin-finding-types` does not compile, which breaks every crate that depends on it and all of the PR's tests. | rust/crates/quoin-finding-types/src/finding.rs:163 |
| FND-002 | high | FR-032-AC-20 is violated. The ladder calls `unrecorded_evidence` before `failed_run`, and each picks the first suite only within its own cause. So `a-suite` (failed) plus `z-suite` (no run) yields `z-suite` with `missing-run`, not the bytewise-first `a-suite` with `failed-run`. `tc_1760_deterministic_stale_suite_selection_failed_first` would fail. | rust/crates/quoin-auditor/src/audit/ladder.rs:77-84 |
| FND-003 | high | FR-115-AC-22 is not implemented. `cause` is a closed serde enum, so an out-of-enum value fails `MatrixInput` deserialization as `BadRequest`, never `Refused` with `invalid-stale-cause`. The stale-evidence branch of the new validation loop is empty, holding only a comment. | rust/crates/quoin-assurance/src/matrix.rs:430-443 |
| FND-004 | high | The `Finding` struct literal in `case_graph.rs` omits the new `cause` field (E0063), so the `quoin-assurance` test target does not compile even after FND-001 is fixed. | rust/crates/quoin-assurance/tests/case_graph.rs:40-53 |
| FND-005 | medium | Regression: the `failed_run` summary no longer names the outcome. `tc_1963_failed_run_stale_evidence.rs` asserts that an errored run's summary contains `error`, but the new text says only "which failed in the recorded run". | rust/crates/quoin-auditor/src/audit/stale.rs:116-127 |
| FND-006 | medium | `-D warnings` lint failures. The `join_suites` import at `stale.rs:19` is now unused, and `ladder::join_suites` is dead code. Production code adds `expect` (`stale.rs:115`) and `[0]` indexing (`stale.rs:58`, `106`, `114`) against the workspace `expect_used`/`indexing_slicing` policy. Also a needless `failed_symbols.clone()`, and `!x.is_some()` in both new test files. | rust/crates/quoin-auditor/src/audit/stale.rs:19 |
| FND-007 | medium | FR-167 requires `open_findings` to be ordered with an absent value before a present one. The snapshot's `cause_order` maps `None` to 2, which sorts an absent cause after `failed-run` and `missing-run`. | rust/crates/quoin-cli/src/release/snapshot.rs:131-137 |
| FND-008 | medium | FR-115 orders stale findings by `cause`, then `path`, `line`, `symbol`, `summary`. After `cause` the code falls back to the derived `Ord` on `FindingDetail` (`kind`, `summary`, `path`, …), so `summary` breaks ties before `path`, `line` and `symbol`. | rust/crates/quoin-assurance/src/matrix.rs:731-744 |
| FND-009 | low | `cause_order` is duplicated verbatim in `matrix.rs` and `snapshot.rs`. Declaring `FailedRun` before `MissingRun` would let the derived `Ord` on `StaleCause` give the spec order and remove both helpers. | rust/crates/quoin-finding-types/src/finding.rs:20-25 |

## Coverage

- Examined: every hunk of the 8 changed files; `ladder.rs:36-90` (call order);
  `case_graph.rs:36-53` and `tc_1963_failed_run_stale_evidence.rs` (existing
  callers and tests of the changed types and summaries).
- Spec read for conformance: FR-032 Behavior and AC-17..21, FR-115 "Typed
  cause" and AC-17..25, FR-167 ordering and cause clauses.
- Gates: scoped `cargo check -p quoin-finding-types` only. No aggregate gate.

## New findings (disposition pass 1)

Found while checking fix round 1 at head `9263c933`. Every one was traced by
reading the code. No build or test was run in this pass.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | high | The `tc_1760_matrix_cause` test target does not compile. It uses `EvidenceStatus::Unevaluated`, which is not a variant. It imports `quoin_core`, which is not a dev-dependency of `quoin-assurance`. It calls `serde_json::to_value(&matrix_input)` although `MatrixInput` derives only `Deserialize`. It also calls `CoreError::code()` and `context()`, but those are public fields, not methods, and `context` holds `String`s, not JSON values. | rust/crates/quoin-assurance/tests/tc_1760_matrix_cause.rs:154 |
| FND-011 | high | The `tc_3604_release_snapshot` test target does not compile. The three new FR-167 tests call `Repo::populated()`, `commit_snapshot`, `read_snapshot` and `HEAD`, and none of these exist in the file. This also takes every existing FR-167 snapshot test in that target down with it. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:1443 |
| FND-012 | medium | Regression: `matrix.rs` `cause_order` now maps `None` to 0, so a stale-evidence finding with no cause sorts first. FR-115 "Typed cause" says to order by `cause` (`failed-run` before `missing-run`, an absent `cause` last). The absent-first rule that FND-007 fixed belongs to FR-167's snapshot only. | rust/crates/quoin-assurance/src/matrix.rs:761-767 |


## New findings (final disposition)

The round-2 Opus disposition at head `467c317c` added these findings; the final permitted fix round addressed both.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | medium | Unknown stale causes passed `parse_report` and later failed canonicalization rather than producing a parse error naming `cause`; the auditor's `Invalid` arm could silently drop the finding. | rust/crates/quoin-graph-analysis/src/model/audit.rs:126, 210-214; rust/crates/quoin-auditor/src/audit/stale.rs:129-133 |
| FND-014 | low | The matrix cause-order comment said an absent cause sorts first, contradicting FR-115 and the implementation. | rust/crates/quoin-assurance/src/matrix.rs:732-733 |

## New findings (aggregate gate)

The final gate first found a compile failure in the quoin-cli target: snapshot.rs imported `StaleCause` from `quoin-finding-types`, which is not a direct quoin-cli dependency.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-015 | high | The workspace does not compile because `quoin-cli/src/release/snapshot.rs` imports `quoin_finding_types::StaleCause` without a direct dependency. | rust/crates/quoin-cli/src/release/snapshot.rs:15 |

## Dispositions

Latest outcomes at PR #932 head `104078c6`, after fix round 3. Focused tests and Clippy passed for quoin-graph-analysis, quoin-auditor, quoin-assurance and quoin-finding-types. The final aggregate gate remains pending.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The stray delimiter is removed from finding.rs; the final workspace gate will verify compilation. |
| FND-002 | fixed | The auditor selects the bytewise-first suite across both stale causes and the ladder calls the combined producer once. |
| FND-003 | fixed | Unknown causes deserialize to the operation-level invalid sentinel and matrix.build/core return Refused with `invalid-stale-cause` naming the obligation; graph parse rejects unknown cause values at `report.findings.N.cause`. |
| FND-004 | fixed | `case_graph.rs` supplies `cause: None` in its Finding literal. |
| FND-005 | fixed | The failed-run summary retains the `fail` or `error` outcome text. |
| FND-006 | fixed | The unused code, expect/indexing/panic lints, must-use lints and same-arm match lint were resolved; focused workspace Clippy passed for the four changed audited crates. |
| FND-007 | fixed | Snapshot ordering places an absent cause before present causes as FR-167 requires. |
| FND-008 | fixed | Matrix stale-finding ordering now sorts cause, path, line, symbol and summary explicitly. |
| FND-009 | fixed | The matrix and snapshot cause-order helpers implement distinct FR-115 and FR-167 ordering rules, so they are no longer duplicates. |
| FND-010 | fixed | The uninspected-suite test uses `Undischarged`; invalid imports, serialization and error-accessor calls were removed. |
| FND-011 | fixed | Snapshot tests use existing helpers and fixtures and assert both typed causes are present. |
| FND-012 | fixed | Matrix cause ordering puts `Invalid`/absent last while valid causes use the required order. |
| FND-013 | fixed | Graph audit parsing rejects unknown cause strings with a member-specific error; the auditor's invalid-sentinel arm now emits a stale finding instead of dropping the suite. Two tagged graph-parse tests cover the boundary. |
| FND-014 | fixed | The matrix ordering comment now states that absent cause sorts after `failed-run` and `missing-run`. |
| FND-015 | fixed | `StaleCause` is publicly re-exported through `quoin_assurance::matrix`; snapshot.rs imports it through that existing direct dependency, avoiding a new crate dependency. Final aggregate gate is pending. |
