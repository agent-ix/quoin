---
id: SR-20261113082616
title: "Gap analysis — VER-106 FR-152 --discharges for differential-report against FR-152, NFR-033, TC-002, FR-041 (quoin PR 856)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-152-analysis-run-entry-link-to-obligation.md, spec/non-functional/NFR-033-analysis-counting-never-infers-a-binding.md, spec/test/TC-002-analysis-result-counting.md, spec/functional/FR-041-sbom-inventory-evidence.md, rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs, rust/crates/quoin-cli/tests/tc_ver80_differential_report_command.rs (context), rust/crates/quoin-cli/src/evidence.rs, rust/crates/quoin-cli/src/evidence/audit.rs (context)"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-152", type: references}
  - {target: "ix://agent-ix/quoin/NFR-033", type: references}
  - {target: "ix://agent-ix/quoin/TC-002", type: references}
  - {target: "ix://agent-ix/quoin/FR-041", type: references}
---

## Summary

Ticket: VER-106. PR head `7c47d6ea`. Plan completion: not assessed. I checked each FR-152
criterion against its tagged test and against the built `quoin` binary in a temporary repository
(the tc_1080 module and spec, obligations FR-001-AC-1..3).

The binary does the following. A record with no `--discharges` binds nothing, and
`quoin evidence audit --json` then reports `{kind: undischarged, obligation: FR-001-AC-1}`. After
`--discharges 'FR-001-AC-2, FR-001-AC-1,NOPE-1'` both obligations bind, `NOPE-1` is unmatched, and
those `undischarged` findings clear. `--discharges A --discharges B` exits 3 with clap's "cannot be
used multiple times". A disagreement with `--discharges` prints the `not transcribed` line. junit
with `--discharges` exits 1 naming the adapter and writes no run.

## Verdict

The behaviour FR-152 needs is in place, and NFR-033 statement 1 holds. Three gaps need closing
before merge. FND-001 is a spec form the CLI does not accept (TC-002 step 2 cannot be run as
written). FND-002 is an AC clause no test asserts. FND-003 is an FR-041 criterion that now
contradicts FR-152.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-152-AC-6 says "`--discharges` given twice", and TC-002 step 2 writes `--discharges FR-001-AC-1 --discharges FR-001-AC-2`. The CLI argument is single-valued (`Arg::new("discharges")`, the default `ArgAction::Set`) and comma-separated, as the shipped help says. Given twice, it exits 3 (reproduced). FR-041 defines `--discharges <obligation-id>` and no repetition, and FR-152 cites FR-041's meaning "and adds nothing else: no new option". So the CLI is right and the spec is wrong. Minimal consistent fix, spec only: in FR-152-AC-6 and TC-002 step 2, use `--discharges FR-001-AC-1,FR-001-AC-2` (one comma-separated value). Switching to `ArgAction::Append` would change the grammar for every adapter and is not needed. The PR's AC-6 test passes an array straight to dispatch, so it covers neither form. Add one CLI case in tc_ver80_differential_report_command.rs using the comma form. | spec/functional/FR-152-analysis-run-entry-link-to-obligation.md:126; spec/test/TC-002-analysis-result-counting.md:27; rust/crates/quoin-cli/src/evidence.rs:69,139-148 |
| FND-002 | medium | FR-152-AC-2's clause "`quoin evidence audit` reports the obligation as undischarged" has no assertion. The audit does behave that way (reproduced above), and NFR-033's first metric is verified through AC-2. A test is feasible in the existing CLI fixture (tc_ver80_differential_report_command.rs already has a module, spec and FR-001-AC-1): record SATISFIED with no `--discharges`, run `quoin evidence audit --repo . --module . --json`, and assert a finding with `kind == "undischarged"` and `obligation == "FR-001-AC-1"`. | rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs:216-229 |
| FND-003 | high | FR-041-AC-9 still reads "`--discharges` on a run adapter other than sbom is refused". That contradicts FR-152's first Behavior SHALL and FR-041's own prose (lines 83-84), which admit differential-report. The AC-9 test (tc_2708, junit) still passes, so nothing flags it. Fix, spec only: "other than sbom and differential-report". | spec/functional/FR-041-sbom-inventory-evidence.md:112 |
| FND-004 | low | Some AC clauses are asserted only at the dispatch level, though the CLI fixture exists. AC-5 requires the `unrepresented` result printed "in human and JSON output" when stated with `--discharges`, but the test checks the payload only (the human line was verified by hand). AC-7 requires a non-zero exit and lists an oversize refusal, but the test checks `is_err()` over empty, v1 and pretty input and has no oversize row. Add the `--discharges` arguments to the existing CLI disagreement and refusal cases. | rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs:251-265,295-314 |
| FND-005 | low | FR-152-CON-1 ("no field to the run record and no store file") is traced on the AC-1 test, which asserts only the entry count and `traceIds`. Nothing would fail if a field or a file were added. Assert the entry's key set and the store's file set, or drop the CON-1 tag. | rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs:194-211 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-152-AC-1 | examined | Covered; symbol not independently checked (SR-20261113082615 FND-002) |
| FR-152-AC-2 | examined | Entry and binding covered; audit clause unasserted (FND-002) |
| FR-152-AC-3 | examined | Covered; comment misstated (SR-20261113082615 FND-003) |
| FR-152-AC-4 | examined | Covered for refuted, inconclusive and unavailable |
| FR-152-AC-5 | examined | JSON payload covered; human output not asserted with `--discharges` (FND-004) |
| FR-152-AC-6 | examined | Behaviour covered at dispatch; spec flag form wrong (FND-001) |
| FR-152-AC-7 | examined | Covered at dispatch; exit code and oversize missing (FND-004) |
| FR-152-AC-8 | examined | Covered; refusal names junit, no run written |
| FR-152-CON-1 | examined | Weak trace (FND-005) |
| FR-152-CON-2 | examined | Holds on inspection; no new I/O |
| NFR-033 statement 1 and metric 1 | examined | Holds; verification depends on AC-2 (FND-002) |
| TC-002 steps 1-2 | examined | Step 1 runs at dispatch; step 2's two-flag invocation fails at the CLI (FND-001) |
| FR-041 `--discharges` prose and AC-9 | examined | Prose updated; AC-9 stale (FND-003) |
