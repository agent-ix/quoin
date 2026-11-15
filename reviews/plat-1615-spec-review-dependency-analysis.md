---
id: SR-20261114002442
title: "Dependency analysis — PLAT-1615 M2 dossier relationship edges (quoin#929)"
type: SpecReview
analysis: dependency
date: 2026-11-15
scope: "agent-ix/quoin; frontmatter relationships and Dependencies sections of spec/functional/FR-191, FR-196..FR-201, spec/non-functional/NFR-041, spec/stakeholder/StR-010"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-199", type: references }
  - { target: "ix://agent-ix/quoin/FR-201", type: references }
---

## Summary

Ticket: PLAT-1615. Compared each new `relationships:` edge with the prose Upstream/Downstream lists, and separated enablement work (refactors, upstream contract extensions) from the feature FRs.

## Verdict

**FAIL** — two upstream dependencies are named in prose but have no relationship edge. The edge pattern matches the M1 FRs (`traces_to` StR-010, `implements` US-024, `extends` FR-191, `references` sources), StR-010 gains `satisfied_by` for all seven new documents, and NFR-041 `constrains` all six FRs. Enablement is named explicitly: PLAT-1760 (typed failed-run cause) for FR-196, PLAT-1761 (quire-cli FR-038 extension) for FR-201, and the shared-builder moves for FR-197 and FR-201.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-199 lists FR-145 as an upstream for policy-cell values but has no `references` edge to FR-145. | spec/functional/FR-199-dossier-risk-management-content.md:5-15,86 |
| FND-002 | low | FR-201 depends on quire-cli FR-038 for the approval input but has no relationship edge to it, while it does carry edges for EA FR-047 and FR-052. | spec/functional/FR-201-dossier-release-record-content.md:5-21,100-101 |

## Coverage

- Examined: every relationships block in the diff and every Dependencies section (FR-191, FR-196..FR-201, NFR-041, StR-010, US-024).
- Edges not verified for target existence across repos (spec-objects-safety FR-101, EA FR-047/FR-052): `quire` link resolution could not be run in this session.

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-199 frontmatter now has a `references` edge to `ix://agent-ix/quoin/FR-145`. |
| FND-002 | fixed | FR-201 frontmatter now has a `references` edge to `ix://agent-ix/quire-cli/FR-038`. |
