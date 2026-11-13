---
id: SR-20261113070101
title: "gap-analysis of the licence-policy slice against FR-150, FR-142, FR-144, FR-041, FR-034, NFR-030 (quoin PR 851)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@503030c1; diff origin/main...HEAD; spec/functional/FR-150-licence-and-ban-policy-findings.md; spec/functional/FR-142-advisory-join-onto-soup-records.md; spec/functional/FR-144-soup-report.md; spec/functional/FR-041-sbom-inventory-evidence.md; spec/functional/FR-034-finding-shaped-evidence.md; spec/functional/FR-033-evidence-format-adapters.md; spec/non-functional/NFR-030-soup-views-state-and-never-derive.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-150"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-142"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "references"
---

# Gap analysis of the licence-policy slice (quoin PR 851)

## Summary

Ticket: PLAT-1671. Frozen head 503030c1. Planless run. The matrix was computed with `quire matrix --scope . --format json` over the PR head worktree. Every trace tag the diff adds was read against the test body it sits on.

Matrix, in scope:

- FR-150: all 16 ACs and FR-150-CON-1 are bound. AC-10, AC-12 and CON-1 are Inspection-method rows (`method-without-symbol`).
- FR-041-AC-10 and FR-034-AC-27 are bound.
- FR-142: AC-28..AC-36 are bound.
- FR-144: AC-18..AC-20 are bound.
- NFR-030-AC-5 is bound.

Untagged in scope: FR-142-AC-19..23, AC-25..27, FR-144-AC-16/17 and NFR-030-AC-4. These are the grype ACs. Grype (FR-149) is not implemented on `main` (no `grype` in any `.rs` file), so they are out of this slice and not a finding against it. FR-142-AC-24, also a grype AC, is now reported tagged, through a wrong tag (FND-001).

Reverse gap: none. Every new production item serves FR-150, FR-142, FR-144, FR-041-AC-10 or FR-034-AC-27. No stubs were found. No `todo!`, `unimplemented!` or placeholder returns.

Semantic review (intent to test to code): not run as a separate fan-out. A reviewer subagent cannot ask the opt-in question. The trace-binding checks below were done by hand.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `tc_1671_022` is tagged `FR-142-AC-24`, a grype AC: "A grype advisory on a record with a current approved disposition … shows resolved". The test has no grype scan; grype is not implemented on `main`. The tag makes the matrix report a grype AC covered by a licence-policy test. Remove `FR-142-AC-24` from the marker. | rust/crates/quoin-cli/src/soup/policy/tests.rs:149 |
| FND-002 | medium | `tc_1671_022` is also tagged `FR-142-AC-8`. AC-8 covers an advisory that is unresolved for each of: an expired fact, an absent fact, a non-approved fact, no clause, a clause not in the binding report, or a clause that does not bind. The test asserts only `NoDispositionClause`, on a policy finding. AC-8 is already bound by `tc_1516` (`advisory/tests.rs:491`). Drop the tag. | rust/crates/quoin-cli/src/soup/policy/tests.rs:149-163 |
| FND-003 | medium | `PolicyStatus::Blocked` renders a seventh row status: "policy findings not assessed: an unmatched licence-policy finding could belong to this record". FR-144 says a row states "exactly one of" six statuses or its findings. FR-142 forbids "no policy findings" when an unmatched no-purl finding could belong to the record. FR-144 names no status for that case. The code's choice is conservative and correct by FR-142, but it is unspecified. FR-144's list needs the status added (spec amendment), or the code needs to map this case to a listed status. | rust/crates/quoin-cli/src/soup/policy.rs:148-149,452-453; rust/crates/quoin-cli/src/soup/report/render.rs:128 |
| FND-004 | low | In `tc_1671_021`, `assert_eq!(after_refusal, read)` joins the same `earlier` scan slice twice through a pure function, so it cannot fail. The "refused recording retains nothing" half of FR-142-AC-30 is asserted nowhere in this test. It holds only through `tc_1671_031` (core). Either assert retention end to end (record a refused policy into a store holding the earlier scan, then join what the store returns), or delete the tautology. | rust/crates/quoin-cli/src/soup/policy/tests.rs:138-143 |
| FND-005 | low | `a_row_states_why_it_has_no_policy_verdict_and_never_allowed` is tagged NFR-030-AC-5, whose second clause is "no record is shown as allowed". No assertion in it, or in any other test tagged NFR-030-AC-5, checks the rendered Markdown or JSON for an `allowed` statement. Add the negative assertion. | rust/crates/quoin-cli/tests/tc_1518_soup_report.rs:1068-1131 |
| FND-006 | low | `tc_1671_012`, tagged FR-150-AC-10, checks the adapter source only for licence names and SBOM keys. AC-10 also requires that the adapter open no other file, run no subprocess and open no network connection. A reviewer grep found no `std::process`, `std::fs`, `std::net` or `Command` in `licence_policy.rs`, but the test does not check them. Add those needles. | rust/crates/quoin-evidence/tests/tc_licence_policy.rs:425-465 |
| FND-007 | low | FR-150-AC-16 says a licence finding's message is "the joined offending texts" (joined with `, `). Every test case has exactly one offending text per finding, so the `join(", ")` path is untested. Add a component with two not-allowed ids. | rust/crates/quoin-evidence/src/adapters/licence_policy.rs:193-207; rust/crates/quoin-evidence/tests/tc_licence_policy.rs:74-123 |

## Verdict

FAIL, by the verdict rule (one high finding). FND-001 is a one-line fix: drop the tag. Apart from the trace defects, coverage of the slice is complete. Every FR-150 AC is bound by a test that exercises the real adapter over real Syft output. The join and report ACs are bound by tests that can fail: each asserts concrete rule ids, statuses, reasons and resolutions, not re-derived values. After FND-001 and FND-002 are fixed and FND-003 is settled, this would be CONDITIONAL on lows.

## Coverage

- FR-150: 16/16 ACs bound; CON-1 bound (Inspection); CON-2 Inspection, unbound and not code-testable.
- FR-041-AC-10: bound (`tc_1671_002`, `tc_1671_003`; the run-entry byte-identity is asserted).
- FR-034-AC-27: bound (`tc_1671_013`: write, read and gc, plus absence).
- FR-142-AC-13 (`tc_1516_009` rewrite): rewritten as FR-142 requires. `rejected` and `banned` are now in neither count; the source and `unclassified` findings give `set_aside` 2; the policy join attaches both.
- FR-142-AC-28..36: bound. FR-144-AC-18..20: bound. NFR-030-AC-5: bound (see FND-005).
- Grype ACs: out of slice (not implemented on `main`).
- Semantic review: skipped (reviewer subagent; trace bindings were checked by hand).
- Plan completion: not assessed
