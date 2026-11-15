---
id: SR-20261114002437
title: "Gap analysis — PLAT-1615 M2 dossier content vs. source entry points (quoin#929)"
type: SpecReview
analysis: gap-analysis
date: 2026-11-15
scope: "agent-ix/quoin; spec/functional/FR-196..FR-201, rust/crates/quoin-assurance/src/matrix.rs, rust/crates/quoin-cli/src/soup/report.rs, rust/crates/quoin-anomaly/src/report.rs, rust/crates/quoin-risk-advice/src/lib.rs, rust/crates/quoin-risk-advice/src/summary.rs, rust/crates/quoin-cli/src/release/snapshot.rs, rust/crates/quoin-cli/src/release/history.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-196", type: references }
  - { target: "ix://agent-ix/quoin/FR-199", type: references }
  - { target: "ix://agent-ix/quoin/FR-201", type: references }
---

## Summary

Ticket: PLAT-1615. Spec-only PR; no production code changed, and the six new FRs are implemented by downstream tickets (PLAT-1617, 1618, 1619, 1620, 1622, 1656), so every new AC is expectedly untested. This pass instead checked each FR's declared source entry point against the code on the branch: does the named library operation exist, and does it carry every value the FR requires the document to render?

## Verdict

**FAIL** — three FRs require rendered values that their declared sources do not carry, so a generator would have to derive them or read records outside the declared source, against the "one implementation of each report" rule. Entry points confirmed present: `quoin_assurance::matrix::build` (matrix.rs:342) with the five Evidence Status values and `reason`; the SOUP builder in quoin-cli (report.rs) with `--binding` required; `quoin_anomaly::build_report` (report.rs:336) with `byHazard`, `byRequirement`, `releaseRationale`, `rationaleChange`; `quoin_risk_advice::summarize`/`advise` (lib.rs:74,123) with `policy_region` and `decided_by`; `quoin_quire::records::read` (records.rs:108); `ChangeRecord` with record id, revision and commit (history.rs:30).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-196 requires a `Newest result` column and "the failed or errored result", but `MatrixCriterion`/`BindingDetail` carry only binding suites and findings, with no run outcome or time. | spec/functional/FR-196-dossier-verification-report-content.md:43,79; rust/crates/quoin-assurance/src/matrix.rs:139-274 |
| FND-002 | medium | FR-199 requires hazard titles, a `Records in no chain` section, benefit-risk records as authored and a source reason for an absent region; `Summary`/`Row` carry ids only and `policy_region: Option<String>` with no reason. | spec/functional/FR-199-dossier-risk-management-content.md:39-41,51-52,60-63; rust/crates/quoin-risk-advice/src/summary.rs:127-192 |
| FND-003 | medium | FR-201 requires the release id, name, baseline name and baseline items, but `Snapshot` holds `baseline_commit`, `previous_release`, coverage, freshness, findings, SOUP, anomalies and change records only. | spec/functional/FR-201-dossier-release-record-content.md:51-52,84; rust/crates/quoin-cli/src/release/snapshot.rs:29-40 |
| FND-004 | low | FR-201 says the snapshot parser is "exposed as a shared read-only library API" without saying where; `Snapshot` is `pub(super)` inside quoin-cli, unlike FR-197 which states the builder moves to a library crate. | spec/functional/FR-201-dossier-release-record-content.md:37-38; rust/crates/quoin-cli/src/release/snapshot.rs:29 |

## Coverage

- Reconciliation: `quoin matrix` could not be run (permission layer requires approval for `quoin`); manual AC-to-test check performed instead: no test in `rust/` carries an FR-196..FR-201 or NFR-041 trace tag, which is expected for a spec-only PR whose implementation tickets are downstream.
- Plan completion: not assessed.
- FR-201-AC-1's EA approver/meaning fields and FR-201's release id/name may come from EA FR-047's result; EA's checkout is outside this session's readable scope, so FND-003 is held at medium confidence.
- Semantic review: skipped; not requested.

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-196 table column is now `Source findings`; the FR states MatrixCriterion carries no newest run outcome or time, and AC-3 forbids inventing one. |
| FND-002 | fixed | FR-199 drops hazard titles, `Records in no chain` and authored benefit-risk; it renders `Advice.decision_queue` (QueueEntry kinds include `benefit_risk`, queue.rs:23) and `policy_cells[].findings` (policy.rs:41) for an absent region. |
| FND-003 | fixed | FR-201 states the snapshot has no release id/name or baseline name/items and takes them only from the EA result's `release`/`baseline` members; EA member names not checked here (EA checkout outside session scope). |
| FND-004 | fixed | FR-201 names a shared `quoin-release-snapshot` library extracted from snapshot.rs, used by CLI and dossier. |
