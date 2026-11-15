---
id: SR-20261115183523
title: "Dependency — PLAT-1760 FR-115 typed stale-evidence cause"
type: SpecReview
analysis: dependency
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md, spec/functional/FR-032-evidence-auditor.md, spec/functional/FR-185-in-toto-test-result-export.md (context); PLAT-1760"
review_set: subset
---

## Summary

Ticket: PLAT-1760. Dependency and ordering check of the FR-115 change in PR
#931. PR #931 changes no `relationships:` entry. FR-115 already `requires`
FR-032 and FR-030, which covers the edge this change uses.

## Verdict

**CONDITIONAL**, with one low finding. The graph is acyclic and the edge
direction is correct (FR-032 → FR-115). The work splits into enablement and
feature: (1) enablement, `Finding.cause` in `quoin-finding-types` with the two
producers setting it in `quoin-auditor` (FR-032's lane), then (2) feature, the
`evidence_detail.findings` passthrough in `quoin-assurance`/`quoin matrix`.
That order is implied but not written down. See finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-115's Dependencies section still describes FR-032 only as "the audit report, including the failed/errored latest-run check PLAT-1086/CR-002 added". It does not name the new upstream prerequisite, that the auditor's `Finding` carries `cause`, so the enablement step that must land before AC-17..19 can pass has no entry in the upstream edge. | spec/functional/FR-115-evidence-backed-test-matrix.md:322-324 |

## Dispositions

Round 1, fix commit 0883d0c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-115 Dependencies now identifies FR-032's typed stale-evidence cause as an upstream input. |
