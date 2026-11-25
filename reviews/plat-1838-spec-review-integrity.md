---
id: SR-967175026302197768900474181941935084763
title: "Integrity review of Quoin PLAT-1838 dossier requirements"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-191-dossier-command.md, FR-229 through FR-232, spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md, spec/stakeholder/StR-010-reviewers-receive-documents-generated-from-records.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-191"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-229"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-230"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-231"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-232"
    type: "reviews"
---

## Summary

The new FRs are linked to US-024 and FR-191, and their IDs, acceptance-criterion IDs, links, and verification-method cells validate. Traceability to stakeholder need is incomplete, and FR-191's selector statement does not enumerate all source records consumed by the new FRs.

## Traceability Matrix

| User story | Requirements | Stakeholder requirement | Verification |
| --- | --- | --- | --- |
| US-024 | FR-229–FR-232 | StR-010 is linked by US-024, but the new FRs do not trace directly to it and StR-010's need/validation coverage still enumerates M1/M2 only. | All new ACs state `Test`; matrix currently marks them untagged pending implementation. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new M3 requirements are not allocated to a stakeholder need that names these documents. | FR-229, FR-230, FR-231, FR-232, StR-010 |
| FND-002 | medium | FR-191's common selector source list omits the decision and architecture records required by FR-230 and FR-231. | FR-191, FR-230, FR-231 |

## Evidence

- FR-229 through FR-232 link to US-024 and extend FR-191, but none has a `traces_to` edge to StR-010. StR-010's need lists the prior requirements, architecture, trace, verification, SOUP, anomaly, risk, and release documents; its validation criteria cover M1/M2. It does not mention the four new M3 documents.
- FR-191 Inputs says the common `--scope` and `--module` select anomaly, risk, threat-model, product, software, and device records. FR-230 consumes `documentation_level_decision` records and FR-231 consumes architecture records (`part`, `port`, `connection`, allocations, and architecture decision records), which that statement does not name.
