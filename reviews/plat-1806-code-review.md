---
id: SR-20261123184241
title: "Code review of PLAT-1806 evidence record exit status and pytest symbol mapping"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-core/src/ops/evidence/records.rs; rust/crates/quoin-core/src/ops/evidence/wire.rs; rust/crates/quoin-cli/src/evidence.rs; rust/crates/quoin-evidence/src/adapters/junit.rs; rust/crates/quoin-evidence/src/adapters/mod.rs; rust/crates/quoin-core/tests/tc_plat1806_record_exit_status.rs; rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs; rust/crates/quoin-cli/tests/tc_plat1806_record_exit.rs; rust/crates/quoin-cli/tests/tc_ver80_differential_report_command.rs (PR #1028)"
review_set: subset
---

# Code review of PLAT-1806 evidence record exit status and pytest symbol mapping

## Summary

Ticket: PLAT-1806. PR agent-ix/quoin#1028 makes `quoin evidence record` exit 1 (`Outcome::Partial`) with one `warning:` stderr line when a trace id is unmatched (FR-030-AC-32) or when a test-case adapter's run has entries and none carries a trace id (FR-030-AC-33). It also maps pytest JUnit cases to the Python extractor's symbol (FR-033-AC-16/17). The review covers code-review and its rust-review lane, read against the repo's rust-style boundary contract. Focused tests were run through the locked build lane: quoin-core `tc_plat1806_record_exit_status`, `tc_458_evidence_boundary`, `tc_plat348_finding_and_sbom_record` and `tc_ver80_differential_report_record`, plus quoin-cli `tc_plat1806_record_exit`, `tc_ver80_differential_report_command`, `tc_1080_matrix_command`, `tc_1597_criticality_mutation_floor` and `tc_3604_release_snapshot`. All of them pass. `cargo fmt --check` is clean. Package-scoped clippy reports nothing in the changed files. Two defects were reproduced against the built binary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-030-AC-33 is bypassed by an entry with an empty `traceIds` array. `recording_warning` tests `trace_ids.is_none()`, and the entries adapter and the store accept `"traceIds":[]`. So `{"entries":[{"symbol":"a","outcome":"pass","traceIds":[]}]}` records, binds nothing and exits 0 with no warning (reproduced). Use `trace_ids.as_ref().is_none_or(Vec::is_empty)` and add the case to the AC-33 test | rust/crates/quoin-core/src/ops/evidence/records.rs:137-140 |
| FND-002 | medium | The pytest `[...]` stripping splits at the last `[`, so a parametrized id that itself contains brackets is truncated wrongly. `classname="t.TestA" name="test_a[x[1]]"` (for example `parametrize("s", ["x[1]"])`) is stored as `TestA.test_a[x` (reproduced), and that symbol no extractor emits. A Python identifier never contains `[`, so split at the first `[` when the name ends in `]`. Add the case to the AC-16 table | rust/crates/quoin-evidence/src/adapters/junit.rs:124-130 |
| FND-003 | low | Unknown `--discharges` ids are handled differently by the two paths. A run adapter (`sbom`, `differential-report`) merges them into `unmatched` and now exits 1. A finding adapter (`sarif`, `cargo-audit`, `cargo-deny`, `osv-scanner`, `licence-policy`, `audit-script`) reports them as `unknown` and still exits 0 with no warning. AC-32 is worded over the "outcome's unmatched list" and the run record, so this may be intended scope. Either state that scans are out of scope in FR-030, or apply the same warning to `RecordPayload::Scan.unknown` | rust/crates/quoin-core/src/ops/evidence/records.rs:150-237 |
| FND-004 | low | Misindented `"obligations": obligations(),` inside the `json!` body of tc_458_070. rustfmt does not format macro bodies, so `fmt --check` does not catch it | rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs:251 |

## Verdict

CONDITIONAL. There is no high finding. FND-001 and FND-002 are real wrong results on edge inputs. Each has a one-line fix and a table-row test, and both should be fixed in this PR before merge. FND-003 needs a decision on spec scope (fix it, or write the scope down). FND-004 is a nit.

What holds:
- **AC-32.** The line format matches the spec: byte-ordered, comma-space joined, and taken from the merged sorted and deduplicated `unmatched`, which includes discharge-unknown ids. The run record and the matched bindings are written inside `with_store` before the response returns. The CLI emits the line once through `emit`'s existing `warning` path. The `--json` stdout carries `unmatched`, and plain text does not. The CLI test asserts exit 1, the exact single stderr line, the JSON `unmatched` list, and the files on disk.
- **AC-33.** The early return guarantees exactly one line. The adapter names match the registry (`entries`, `junit`, `agent-eval`, `contract-conformance`). A zero-entry run is excluded, as AC-34 requires.
- **AC-34/AC-35.** Exit 0 is preserved. The tests pass on the old code too, which is expected for exit-0 criteria. They still discriminate against the two plausible wrong implementations: "bound is empty → exit 1" fails AC-34's all-failed run, and "AC-33 for every run adapter" fails AC-35's cargo-mutants and sbom recordings. That second one is the defect the spec review (base FND-001) removed. These oracles are acceptable as they are.
- **FR-033-AC-16/17.** The `--tool` match is the same lower-cased substring rule as adapter selection. The classname==name rule applies first. Module segments before the first `Test*` segment are dropped. An empty classname and a classname with no `Test*` segment give the bare name. Other tools keep the `::` rule, and the test asserts that. `unittest.TestCase` and custom `python_classes` fall to the module-level rule, as the spec puts them out of scope.
- **Modified fixtures.** The tc_458 tests (050/070/120/160) cover the store, gc and audit inputs, not exit status. Their `ok()` helper requires `Outcome::Ok`, so giving the fixture case a trace id and an obligation keeps what they test, and no regression is hidden. The tc_ver80 comma-separated test now expects exit 1 because `NOPE-1` is an unmatched id under AC-32. Its payload assertions on `bound` and `unmatched` are unchanged.
- **`warning` in the payload.** It is an optional, `skip_serializing_if` field on `RecordPayload::Run`. No committed schema snapshot exists. The same `warning` payload convention is already used by report.rs, measurement.rs, completeness.rs and plugin.rs. The spec requires `unmatched` on `--json` stdout and does not forbid extra fields. There is no contract break.
- **Exit taxonomy.** `Partial` (1) carries the payload, and `carries_payload()` callers are unaffected. No internal caller dispatches `evidence.record` and asserts `Ok`. One consumer impact is worth knowing: the Makefile `evidence` target runs `quoin evidence record ... || exit 1`, so an unmatched id in the repo's own traced nextest report now fails `make evidence`. That is the signal the spec asks for.

## Examined

FR-030-AC-32, FR-030-AC-33, FR-030-AC-34, FR-030-AC-35, FR-033-AC-16, FR-033-AC-17 (examined). FR-030 exit-status SHALL, FR-033 pytest mapping procedure, FR-041-AC-8, repo rust-style boundary contract (context only).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit 9a96a99e: `recording_warning` now tests `trace_ids.as_ref().is_none_or(Vec::is_empty)`. The new `tc_1806_an_empty_trace_id_array_does_not_count_as_carrying_one` asserts Partial and the AC-33 line for `"traceIds":[]`. |
| FND-002 | fixed | Commit 9a96a99e: `pytest_symbol` splits at the first `[` (`split_once('[')`). The AC-16 table adds `("t.TestA", "test_a[x[1]]", "TestA.test_a")`, which passes. |
| FND-003 | accepted-no-change | FR-030-AC-32 as merged keys on the run outcome's `unmatched` list. Finding adapters report stated-but-unknown ids in the scan payload's separate `unknown` field (FR-034/FR-041), which AC-32 does not name. Exiting 1 for scans would go beyond the merged ACs. The inconsistency is real but sits outside this PR's criteria, and the coder chose to keep the separate mechanism. |
| FND-004 | fixed | Commit 9a96a99e re-indents the `"obligations": obligations(),` line in tc_458_070 to match its siblings. |
