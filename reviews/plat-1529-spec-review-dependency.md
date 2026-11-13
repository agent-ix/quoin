---
id: SR-20261113084432
title: "Dependency review of FR-165 relationships"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; PR #858 (git diff origin/main...plat-1529-spec-quoin at 87825c1e): spec/functional/FR-165-matrix-design-column.md, spec/functional/FR-115-evidence-backed-test-matrix.md (CR-004 note), spec/functional/index.md"
relationships:
  - target: ix://agent-ix/quoin/FR-165
    type: references
---

# Dependency review of FR-165 relationships

## Summary

Ticket: PLAT-1529. Checks FR-165's `relationships:` edges against its body and Dependencies. The quire-rs FR-092 `requires` edge and the quire-cli FR-043 `references` edge use the renumbered ids and match the companions' Downstream lines (both name quoin FR-165). FR-092 is in open quire-rs PR 670, so the three PRs land together.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-165 `traces_to` StR-004 ("Review and planning run as governed workflows"), but StR-004's PLAT-1080 CR note removes the matrix from that StR: "the test matrix is retired from the workflows this StR governs ... Only review and planning remain governed workflows". The edge, inherited from FR-115, traces to a need that excludes this feature. StR-009 ("Acceptance rests on tests that ran and passed") is the stakeholder need the matrix serves. | spec/functional/FR-165-matrix-design-column.md:6-7 |
| FND-002 | low | Dependencies lists quire-rs FR-086 as Upstream, but `relationships:` has no FR-086 edge. | spec/functional/FR-165-matrix-design-column.md:5-15,94 |

## Examined scope

```yaml
scope:
- id: FR-165-REL
  path: spec/functional/FR-165-matrix-design-column.md
  role: examined
  excerpt: 'relationships: StR-004 traces_to; FR-115 extends; quire-rs FR-092 requires (1:1); quire-cli FR-043 references. Upstream: FR-115; quire-rs FR-092, FR-086; quire-cli FR-043'
- id: FR-165
  path: spec/functional/FR-165-matrix-design-column.md
  role: context_only
  excerpt: '`quoin matrix` SHALL carry that design column through beside the evidence status, so one view reads requirement, design element, test and result, with the evidence freshness of FR-115 as the result.'
```

## Verdict

Changes requested: 0 high, 1 medium, 1 low.
