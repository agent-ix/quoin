---
id: SR-20261114002443
title: "Evidence analysis — PLAT-1615 M2 dossier verification methods (quoin#929)"
type: SpecReview
analysis: evidence
date: 2026-11-15
scope: "agent-ix/quoin; Acceptance Criteria tables of spec/functional/FR-191, FR-196..FR-201, spec/non-functional/NFR-041"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-197", type: references }
  - { target: "ix://agent-ix/quoin/FR-201", type: references }
---

## Summary

Ticket: PLAT-1615. Checked whether each new AC's declared verification method fits the obligation and would produce evidence on a normal run. `quoin advise` could not be run (permission layer), so the methods were judged by reading the ACs.

## Verdict

**FAIL** — one AC is evidenced only by an ignored test, and one Inspection AC holds a clause a test could check. The other 43 new ACs are fixture-level `Test` criteria with observable outcomes. NFR-041-AC-2 is an appropriate `Inspection` of output models.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-201-AC-7 is verified by "an ignored real-tool export test"; an `#[ignore]`d test does not run in the normal suite, so the AC has no evidence unless someone runs it by hand. | spec/functional/FR-201-dossier-release-record-content.md:90 |
| FND-002 | low | FR-197-AC-7 is `Inspection`, but its second clause ("a source row added by that builder appears once in each output") can be tested and is weaker as an inspection. | spec/functional/FR-197-dossier-soup-content.md:73 |

## Coverage

- Examined: all 47 new or edited ACs listed in the scope documents, plus StR-010-VC-1 and VC-4.

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-201-AC-7 now requires an unignored integration test with fixture EA results. |
| FND-002 | fixed | FR-197-AC-7 verification changed from `Inspection` to `Test`. |
