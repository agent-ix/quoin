---
id: SR-20261123184242
title: "Gap analysis of PLAT-1806 evidence record exit status and pytest symbol mapping"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; FR-030-AC-32..35, FR-033-AC-16..17; PR #1028 diff under rust/crates/"
review_set: subset
---

# Gap analysis of PLAT-1806 evidence record exit status and pytest symbol mapping

## Summary

Ticket: PLAT-1806. This is a planless gap analysis scoped to PR agent-ix/quoin#1028. `quire matrix --scope . --format json` reports all six criteria as `tagged`. FR-030-AC-32 is bound by a core test and a CLI test. FR-030-AC-33, -34 and -35 and FR-033-AC-16 and -17 are each bound by a test in `rust/crates/quoin-core/tests/tc_plat1806_record_exit_status.rs`. Every tagged test was run and passes.

**Reverse gap.** Every new production item is owned by these criteria: `recording_warning` and `TEST_CASE_ADAPTERS` by AC-32/33, `RecordPayload::Run.warning` and the CLI `render` pass-through by AC-32, and `parse_junit_for_tool`, `is_pytest` and `pytest_symbol` by FR-033-AC-16. No unowned behaviour was added.

**Stubs.** No stub or coverage inflation was found. Each test asserts a concrete outcome, warning text, payload field or stored symbol.

## Verdict

CONDITIONAL. There are no untagged criteria and no high findings. One coverage gap is low: FR-030-AC-33 names four adapters, and only two of them are exercised. The code-review SR for this PR records the two medium code defects whose fixes need new test rows: an empty `traceIds` array under AC-33, and nested brackets under AC-16.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-030-AC-33 lists `entries`, `junit`, `agent-eval` and `contract-conformance`. Tests exercise only `entries` (default adapter) and `junit` (the pytest AC-17 case). A `contract-conformance` run of legacy rows without `trace_ids`, the realistic traceless case, is not tested, so a name typo in `TEST_CASE_ADAPTERS` for the untested two would go unnoticed | rust/crates/quoin-core/tests/tc_plat1806_record_exit_status.rs:85-97; rust/crates/quoin-core/src/ops/evidence/records.rs:120 |

## Coverage

- Criteria in scope: 6. Tagged: 6. Untagged: 0. Tagged only by ignored tests: 0.
- Tests run: the 7 tests in tc_plat1806_record_exit_status and the 1 in tc_plat1806_record_exit, all passing.
- Semantic review: performed inline by the reviewer as part of the code-review lane; not run as a separate fan-out.
- Plan completion: not assessed
