---
id: SR-20261113104839
title: "Gap analysis — PLAT-1585/PLAT-1586 anomaly report against FR-162, FR-163, FR-164, NFR-035, FR-136 AC-13..16 (quoin PR 865)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-162-anomaly-report.md, spec/functional/FR-163-anomaly-report-release-rationale.md, spec/functional/FR-164-anomaly-report-gate.md, spec/non-functional/NFR-035-anomaly-report-states-and-never-decides.md, spec/functional/FR-136-anomaly-record-mapping.md, rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs, rust/crates/quoin-cli/tests/tc_162_anomaly_report_command.rs, rust/crates/quoin-anomaly/src/record.rs, rust/crates/quoin-anomaly/src/report.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-162", type: references}
  - {target: "ix://agent-ix/quoin/FR-163", type: references}
  - {target: "ix://agent-ix/quoin/FR-164", type: references}
  - {target: "ix://agent-ix/quoin/NFR-035", type: references}
  - {target: "ix://agent-ix/quoin/FR-136", type: references}
---

## Summary

Tickets: PLAT-1585, PLAT-1586. PR head `a64728e3`. Plan completion: not assessed.
`quire matrix --scope .` (quire 0.44.0, engine 0.59.1) reports every criterion of FR-162
(AC-1..14), FR-163 (AC-1..9), FR-164 (AC-1..12), NFR-035 (AC-1..5) and FR-136 (AC-13..16) as
`tagged`. The exception is FR-162-AC-12, an Inspection criterion (`method-without-symbol`). I
inspected it: `git grep` finds no shipped spec-artifacts-process document in the tree, the tests
read `tests/fixtures/anomaly` in place, and the snapshots hold rendered output only. AC-12 holds.

I read each tagged test against its criterion and checked the fixtures it uses in the sibling
checkout (`ed574c9`, merged PR #148). The focused suites pass (quoin-anomaly 27 + 15, quoin-cli
9 + 6). The findings are criteria whose tagged test does not assert all of the criterion, plus one
production behaviour with no test. The derived-commit defect is in the code-review SR
(SR-20261113104838 FND-001). Its missing test is FND-001 here.

## Verdict

Every criterion has a binder, and most of the bindings assert their criterion. Ten gaps remain:
one high, three medium and six low. All are test additions or tightenings, and none needs a spec
change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-162-AC-14 and FR-163-AC-5 are asserted only over repositories where the commit under test is the only similar file, or the first one. No test commits two similar anomaly files separately and checks that each `fixingChange`/`rationaleChange` names that file's own commit. That is why the copy-following defect (code-review FND-001) passes, and why the snapshot test enshrines it (AN-006 and AN-011 report AN-004's commit). Fix: add that case in `tc_162_anomaly_report.rs` and regenerate the snapshots after the fix. | rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:439-533,610-632; rust/crates/quoin-cli/tests/tc_162_anomaly_report_command.rs:223-233 |
| FND-002 | medium | FR-164-AC-5 names "an `impacting` anomaly `closed` as ... `not-a-defect`". The test uses the shipped `closed-not-a-defect` fixture, whose `safety_impact` is `none`. So it passes on the impact alone, and the not-a-defect branch of the carried test is never exercised with `impacting`. Fix: derive an `impacting` variant in memory (`set_key(..., "safety_impact", Some("safety_impact: impacting"))` plus a Hazard row), and assert no `unjustified-impacting` failure. | rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:861-882 |
| FND-003 | medium | NFR-035-AC-4 needs a fixture with every optional value absent, and it needs each printed value checked to be `not stated`, `null`, `none`, an absent marker or a copied value. The tagged test uses `rationale-assessed-without`, which carries `severity: major` and three assessment rows, and it checks only the release-rationale cell and four words. Fix: use `identified` (no severity, resolution, assessment or rationale), and assert every cell of its Markdown row and every value of its JSON element against that allowed set. | rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:561-578 |
| FND-004 | medium | The widened reader adds two new mapping errors: a missing `title`, and a malformed `## Release Rationale` table (`columns`, `row-arity`, `min-rows`). They change both the report's `unreadable` and `quoin anomaly check`'s `anomaly.no-record`. No test covers either. The shipped fixtures `rationale-bad-headers`, `rationale-no-rows`, `rationale-bare-heading` and `rationale-heading-prose` are ready to use. Fix: assert each one's `unreadable` code, and one `anomaly.no-record` from `check_document`, in the FR-162-AC-9 test. | rust/crates/quoin-anomaly/src/record.rs:320-337; rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:340-364 |
| FND-005 | low | FR-163-AC-9 asks that the signature letters map "exactly as FR-136-AC-12 states, in `rationaleChange`". The tagged test sees only `N`/`unsigned`. The path shares `Signature::from_code`, which FR-136-AC-12's test covers, so the risk is small. The binding still asserts one letter of eight. Fix: an in-crate unit test of `Change::of_commit` over all eight letters, or drop the FR-163-AC-9 tag from this test and bind it where the letters are asserted. | rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:608-632 |
| FND-006 | low | NFR-035-AC-2 says the stem "accept" appears "only in values copied from a record". The test drops every line that starts with `\|`, which includes the tool's own cells (`Flag`, state words, `not stated`). It also covers three fixtures, not "every fixture". Fix: check the stem cell by cell, allowing it only in cells copied from the record (title, assessment, rationale), and run over the CLI corpus as well. | rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:942-976 |
| FND-007 | low | FR-162-AC-14 needs all five values `null` for `pending` and `not-applicable` too. The test checks all five only for `absent`. It checks `commit` alone for `pending`, and nothing for `not-applicable`. Fix: loop the five keys over all three non-present states. | rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:497-527 |
| FND-008 | low | FR-163-AC-6's "none yields a diagnostic" and FR-136-AC-15's "none of these yields a diagnostic" are not asserted. The first is not asserted at all. The second is asserted only for the not-under-git case. Fix: assert `diagnostics.is_empty()` for each `fixing(...)` case, and an empty stderr from `quoin anomaly report` over the pending/absent/not-applicable corpus. | rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:634-655,745-766 |
| FND-009 | low | Some clauses are partly asserted. FR-164-AC-1 says "names both by id and path", but only the first failure's path is checked. FR-164-AC-8 says "the document is named in `unreadable`", but only the length is checked. Fix: assert the second path and the unreadable `path`. | rust/crates/quoin-anomaly/tests/tc_162_anomaly_report.rs:780-810,920-930 |
| FND-010 | low | FR-164-AC-10 asserts only `status != 0`. The repo's exit taxonomy gives a parser refusal exit 3 (`EXIT_INVALID`), and pinning it stops a regression to 1 (Partial), which would read as "report with failures". Fix: `assert_eq!(out.status.code(), Some(3))`. | rust/crates/quoin-cli/tests/tc_162_anomaly_report_command.rs:356-368 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-162-AC-1 .. AC-11, AC-13 | examined | Asserted; AC-14 FND-007; AC-9 FND-004 |
| FR-162-AC-12 (Inspection) | examined | Holds: no copy of the documents in the tree |
| FR-162-AC-14 | examined | FND-001, FND-007 |
| FR-163-AC-1 .. AC-4, AC-7, AC-8 | examined | Asserted |
| FR-163-AC-5 | examined | FND-001 |
| FR-163-AC-6 | examined | FND-008 |
| FR-163-AC-9 | examined | FND-005 |
| FR-164-AC-2 .. AC-4, AC-6, AC-7, AC-9, AC-11, AC-12 | examined | Asserted |
| FR-164-AC-1, AC-8 | examined | FND-009 |
| FR-164-AC-5 | examined | FND-002 |
| FR-164-AC-10 | examined | FND-010 |
| NFR-035-AC-1, AC-3, AC-5 | examined | Asserted |
| NFR-035-AC-2 | examined | FND-006 |
| NFR-035-AC-4 | examined | FND-003 |
| FR-136-AC-13, AC-14, AC-16 | examined | Asserted |
| FR-136-AC-15 | examined | FND-008 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (head 82010493): `similar_files_committed_separately_each_keep_their_own_history` commits three near-identical anomalies one commit each. It asserts that each one's `rationaleChange` and `fixingChange` is its own commit, that there are no diagnostics, and one `statusHistory` entry each. The snapshots are regenerated with the true commits. It does not cover the move-plus-add commit (code-review FND-007). |
| FND-002 | fixed | Round 1: the FR-164-AC-5 test now commits `closed-not-a-defect` with `safety_impact: impacting` and still expects only AN-006 to fail. |
| FND-003 | fixed | Round 1: the new `with_every_optional_value_absent_each_printed_value_reads_as_absent_or_is_copied` uses `identified`. It asserts null/`[]`/absent for every optional JSON value and checks every Markdown cell outside the copied ID/Title/Path/Status/Safety-impact columns against `not stated`/`none`/`absent`/`unassessed`. |
| FND-004 | fixed | Round 1: `a_missing_title_or_a_malformed_rationale_table_is_a_mapping_error` asserts the `unreadable` code and an `anomaly.no-record` for a missing title, `rationale-bad-headers` (columns), `rationale-no-rows`, `rationale-bare-heading` and `rationale-no-delimiter` (min-rows). |
| FND-005 | fixed | Round 1: an in-crate unit test `every_signature_letter_maps_in_the_rationale_change_as_in_the_status_history` runs `Change::of_commit` over G/U/X/Y/N/B/R/E. The FR-163-AC-9 tag moved to it. |
| FND-006 | fixed | Round 1: `assert_accept_only_in_copied_values` checks cell by cell, allowing the stem only in the title, assessment, affected and rationale cells. It runs over eight fixtures, with and without the gate. |
| FND-007 | fixed | Round 1: `assert_all_null` runs over absent, pending and both not-applicable values. |
| FND-008 | fixed | Round 1: `pending_absent_and_not_applicable_rationale_changes_yield_no_diagnostic` is new, `fixing()` now returns the diagnostic codes and asserts them empty, and the CLI snapshot test asserts an empty stderr. |
| FND-009 | fixed | Round 1: the second failure's path (`spec/anomalies/AN-002.md`) and the unreadable path are asserted, in the struct and in the JSON. |
| FND-010 | fixed | Round 1: `assert_eq!(out.status.code(), Some(3), "a parser refusal, not a gate failure")`. |
