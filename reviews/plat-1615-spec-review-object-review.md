---
id: SR-20261114002441
title: "Object review — PLAT-1615 M2 dossier source objects (quoin#929)"
type: SpecReview
analysis: base
date: 2026-11-15
scope: "agent-ix/quoin; spec/functional/FR-198, FR-201; rust/crates/quoin-anomaly/src/report.rs, rust/crates/quoin-cli/src/release/snapshot.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-198", type: references }
  - { target: "ix://agent-ix/quoin/FR-201", type: references }
---

## Summary

Ticket: PLAT-1615. Spec object review. Checked that each domain object the new FRs render (`MatrixCriterion`, SOUP rows, anomaly `Report`/`ReportRow`, risk `Row`, `acceptability_policy`, release `Snapshot`/`ChangeRecord`, EA release-verify result) is named with its real members and that no member the source carries is silently dropped.

## Verdict

**FAIL** — two source objects have members the documents neither render nor explicitly exclude. Member names cited in FR-196 (`id`, `method`, `evidence_status`, `evidence_detail`, `analysis_evidence`), FR-198 (`byHazard`, `byRequirement`, `safetyImpact`, `releaseRationale`, `rationaleChange`), FR-199 (`policy_region`) and FR-201 (`change_records` as record, revision, commit) match the code. Missing-value members are covered in the Outputs sections, and the "extra" members in FR-196..FR-201 are covered by gap-analysis SR-20261114002437.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-198 renders `byHazard` and `byRequirement` but not the source's `ungrouped` list (anomalies in no group, with reason `unassessed` or `no-affected-id`), and does not say it is intentionally omitted. | spec/functional/FR-198-dossier-unresolved-anomalies-content.md:38-40,50-52; rust/crates/quoin-anomaly/src/report.rs:180 |
| FND-002 | medium | FR-201 lists an `## Evidence at release` section but never says which snapshot members fill it (`coverage`, `evidence_freshness`, `open_findings`, `soup`, `anomalies`), although FR-201-AC-5 tests "evidence counts" in it. | spec/functional/FR-201-dossier-release-record-content.md:48-55,88; rust/crates/quoin-cli/src/release/snapshot.rs:34-38 |

## Coverage

- Examined: FR-196 Inputs; FR-198 Inputs and Outputs; FR-199 Inputs and Outputs; FR-200 Inputs; FR-201 Inputs, Outputs and AC-5.
- Context only: matrix.rs, summary.rs, history.rs. Spec-objects-safety FR-101 and EA FR-047 are outside this session's readable scope, so their members were not checked.

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-198 adds an `## Ungrouped` section reproducing every source `ungrouped` row with its reason, tested by FR-198-AC-8. |
| FND-002 | fixed | FR-201 Outputs and Behavior now name `coverage`, `evidence_freshness`, `open_findings`, `soup` and `anomalies` as the `Evidence at release` content. |
