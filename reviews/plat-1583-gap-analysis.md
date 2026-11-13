---
id: SR-20261113150301
title: "Gap analysis — PLAT-1583 FR-175 against its 15 criteria, FR-136, FR-030, FR-120, NFR-035 (quoin PR 874)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-175-anomaly-check-fix-verification-evidence.md, spec/functional/FR-136-anomaly-record-mapping.md, spec/functional/FR-030-evidence-store.md, spec/functional/FR-120-trace-marks-in-test-result-output.md (context), spec/non-functional/NFR-035-anomaly-report-states-and-never-decides.md, rust/crates/quoin-anomaly/src/verification.rs, rust/crates/quoin-cli/src/anomaly.rs, rust/crates/quoin-cli/tests/tc_175_fix_verification_command.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-175", type: references}
  - {target: "ix://agent-ix/quoin/FR-136", type: references}
  - {target: "ix://agent-ix/quoin/FR-030", type: references}
  - {target: "ix://agent-ix/quoin/NFR-035", type: references}
---

## Summary

Ticket: PLAT-1583. PR head `ed393a84`. Plan completion: not assessed. Each of the 15 FR-175 criteria
has one `#[trace("FR-175-AC-n")]` test in `tc_175_fix_verification_command.rs`, all 15 pass. To check
the tests can fail, 11 single-line mutants of `verification.rs` and `check.rs` were run against that
file in a scratch worktree; all 11 were killed: skip counted as pass (AC-3, AC-14), unreadable suite
not excluded (AC-6), any resolution evaluated (AC-11), oldest run used (AC-4, AC-8), missing
Criterion reported (AC-7), indirect route removed (AC-8), dangling evaluated (AC-12), Criterion
naming a document evaluated (AC-10), severity `error` (11 tests), `error` outcome not failing (AC-2),
first pass wins (AC-5, AC-9).

AC-15: no anomaly document or corpus copy is committed (the diff is 9 Rust/lock files); each test
copies the shipped corpus into a tempdir at run time, reads the module from the checkout in place,
and canonicalises the checkout path so `--module` carries no `..`. FR-136: no-record mapping and
the error exit are unchanged (tc_136 and tc_162 pass; AC-12 asserts `anomaly.no-record` and exit 1).
NFR-035: AC-14 asserts identical scope/store bytes, identical output across two runs, and none of
"verified", "accepted", "acceptable"; the messages state store facts only.

## Verdict

The requirement is implemented and every criterion is backed by a test that fails when the behaviour
breaks. Two code-review findings are also requirement gaps and are recorded there, not repeated
here: SR-20261113150300 FND-001 (`anomaly.obligations_unavailable` has no owning requirement and no
test) and FND-002 (FR-030-AC-13's report of an unreadable run file is not produced and AC-6's test
does not ask for it). A `Test` row whose target resolves to an acceptance criterion or document is
evaluated, and a `Criterion` row naming a test case is not: both follow FR-175's "evaluate every
`Test` row" and "evaluate a `Criterion` row whose `Target` is an acceptance-criterion id", and the
kind mismatch stays `quire validate`'s (FR-015-AC-15, FR-136). One low spec-text defect follows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-175 says a `Criterion` row naming "an `FR`, `NFR` or hazard document, which FR-015 allows" is not evaluated. FR-015 allows only `FR`/`NFR` documents for a `Criterion` row; FR-015-AC-15 makes a `Criterion` row targeting the hazard a kind mismatch. The code's behaviour (not evaluated) is harmless either way, but the FR-175 statement and AC-10 misstate FR-015. Fix in a later FR-175 edit: drop "or hazard" from the "which FR-015 allows" clause, or say the hazard case is a kind mismatch left to `quire validate`. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:55 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-175-AC-1 .. FR-175-AC-14 | examined | Tagged, passing, mutant-checked |
| FR-175-AC-15 (Inspection) | examined | Holds |
| FR-175 statement on document-target Criterion rows | examined | FND-001 |
| FR-136 record reader, no-record, exit rule | examined | Unchanged behaviour; CR note widening implemented |
| FR-030 newest-by-timestamp, AC-13, AC-15 | examined | Newest and AC-15 correct; AC-13 see code review FND-002 |
| FR-120 run entry trace ids and outcomes | context_only | Read for the match rule |
| NFR-035 states, never accepts, read-only, deterministic | examined | Holds (AC-14) |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | No test asserts that a store I/O error fails the check: replacing `latest_run(..).map_err(..)?` with `.unwrap_or(None)` leaves tc_175 and tc_162 passing. The behaviour is right (reproduced by hand with a mode-000 run file) but unguarded. Add a test with an unreadable-by-permission run file, or a store path that is a file, asserting a non-zero exit. | rust/crates/quoin-anomaly/src/verification.rs:199-200 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-175 now states FR/NFR document rows as FR-015 allows and hazard rows as the FR-015-AC-15 kind mismatch `quire validate` reports. |
| FND-002 | fixed | Round 2 (`d8fd09c7`): `a_run_file_the_store_cannot_read_fails_the_check_instead_of_being_skipped` makes a run file mode 000 and asserts a non-zero exit and neither code; the swallowed-I/O mutant (N6) now fails it. It returns early where the mode does not bind (root), the repo's existing pattern for permission tests. |
