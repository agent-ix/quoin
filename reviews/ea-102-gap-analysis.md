---
id: SR-20261113111445
title: "Gap analysis — EA-102 quoin release snapshot against FR-166, FR-167, FR-168, NFR-036 (quoin PR 866)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-166-release-snapshot-command.md, spec/functional/FR-167-release-snapshot-format.md, spec/functional/FR-168-release-snapshot-change-records.md, spec/non-functional/NFR-036-release-snapshot-is-deterministic-and-never-decides.md, spec/functional/FR-144-soup-report.md (context), spec/functional/FR-162-anomaly-report.md (context), rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs, rust/crates/quoin-cli/src/release/tests.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-166", type: references}
  - {target: "ix://agent-ix/quoin/FR-167", type: references}
  - {target: "ix://agent-ix/quoin/FR-168", type: references}
  - {target: "ix://agent-ix/quoin/NFR-036", type: references}
---

## Summary

Ticket: EA-102. PR head `63155109`. Planless; plan completion: not assessed. The installed
`quoin matrix --repo .` (0.30.1) reports these criteria `untagged`, and 1518 of the repository's
criteria are `untagged` repo-wide, including long-tagged ones such as FR-144. The matrix's
static axis does not see this repository's `#[trace]` attributes, so the matrix could not confirm
any binding. The AC-to-test mapping below was checked by hand from the `#[trace]` attributes and
the test bodies.

## Verdict

Every Test criterion of FR-166 and FR-168 has a test that can fail. FR-167 is covered except two
cases of FR-167-AC-4 (FND-001). The NFR-036 tags bind no obligation (FND-002). Two FR-167 spec
points need a spec edit (FND-003, FND-004). EA-102 AC-2 is covered for stale, suspect and
undischarged, but not for a failed run (FND-005).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two cases of FR-167-AC-4 are untested: a scan reported unordered, and a policy finding matching no record. The `scan_unordered` arms of `scan_state`/`licence_state` and the `unmatched_policy_finding` loop have no test. The fixture yields only `scan_not_run` and `unmatched_advisory`. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:782-825; rust/crates/quoin-cli/src/soup/report/snapshot.rs:188-235 |
| FND-002 | low | Two tests are tagged `#[trace("NFR-036")]`, but NFR-036's obligations are NFR-036-M-1 to M-4, so the tags bind nothing. Tag the metric rows the tests measure (M-1 bytes, M-2 environment/path, M-3 verdict keys, M-4 files changed). | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:456,1039 |
| FND-003 | low | Spec: FR-167 lists the `not_assessed` kind `scan_refused` (state word `refused`). FR-144 never distinguishes it: a log refused at recording renders as not run (FR-144-AC-5, and render.rs "or its log was refused at recording"). No source can produce the kind. Drop it from FR-167, or have FR-144 state it. | spec/functional/FR-167-release-snapshot-format.md:34-38 |
| FND-004 | low | Spec: FR-167's `coverage` row names `id, static_status, evidence_status` per requirement, per criterion. The implementation nests them as `{id, criteria: [{id, static_status, evidence_status}]}`, mirroring FR-115. `criteria` is a member the table does not name, which a strict reading of "SHALL NOT carry a member that the table does not name" forbids. Name the shape in FR-167. | spec/functional/FR-167-release-snapshot-format.md:29; rust/crates/quoin-cli/src/release/snapshot.rs:42-56 |
| FND-005 | low | EA-102 AC-2 says "failed, suppressed or stale evidence is reported as such". The fixtures cover stale, undischarged and no-run end to end, and suspect in a unit test. No fixture records a failing testcase, so the snapshot of a failed run is untested. | rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:187-228 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-166-AC-1 | examined | one_file_is_written_with_exactly_the_named_members_and_nothing_else_changes |
| FR-166-AC-2 | examined | the_bytes_are_the_same_without_git_and_with_anomaly_files_that_have_history (PATH empty, git history) |
| FR-166-AC-3 | examined | integration test plus a unit test |
| FR-166-AC-4 | examined | an_existing_out_is_refused_and_left_unchanged |
| FR-166-AC-5 | examined | missing history and malformed binding |
| FR-166-AC-6 | examined | matrix and soup report compared member by member. Anomalies are compared to authored values, because `quoin anomaly report` is not on main |
| FR-166-AC-7 | examined | Inspection: no git, gix or hashing in release/* (code review) |
| FR-166-AC-8 | examined | support moves unknown EoS, and `--records` is read |
| FR-166-AC-9 | examined | anomaly_unreadable with path/code; history_invalid |
| FR-167-AC-1 | examined | unit test, seven positions |
| FR-167-AC-2 | examined | five keys, sum |
| FR-167-AC-3 | examined | statuses; ordering in a unit test with reversed input |
| FR-167-AC-4 | examined | FND-001 |
| FR-167-AC-5 | examined | three anomalies, rationale byte-equal, closed fixed/duplicate excluded |
| FR-167-AC-6 | examined | empty store |
| FR-167-AC-7 | examined | key/value scan in the NFR-036 test |
| FR-167-AC-9 | examined | with the FR-166-AC-2 test |
| FR-168-AC-1..AC-6 | examined | all present (integration and unit) |
| NFR-036 M-1..M-4 | examined | measured by the two tagged tests; FND-002 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The FR-166 clause added for an unreadable retained scan has no acceptance criterion. Its test, and the new failing-run test, are tagged `#[trace("EA-102")]`, a Linear ticket id that binds no obligation. Add an FR-166 AC for the refusal, tag the scan test to it, and tag the failing-run test to FR-167-AC-3 or an FR-115 criterion. | spec/functional/FR-166-release-snapshot-command.md:85-86; rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:1328,1346 |

## Dispositions

Round 1, head `c95917ed`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
