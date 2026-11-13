---
id: SR-20261113052921
title: Dependency review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)
type: SpecReview
analysis: dependency
review_set: subset
scope: 'agent-ix/quoin; PR #846 (gh pr diff 846 at 888cbecb): spec/functional/FR-149-grype-advisory-report-adapter.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052921: Dependency review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)

## Summary

Ticket: PLAT-1601. New `relationships:` edges in PR #846: FR-149 traces_to StR-009, extends FR-033 and FR-034, references FR-041, FR-142 and engineering-assurance FR-044; FR-142 depends_on FR-149 and references EA FR-044; FR-144 references FR-149. Edge direction and types agree with the Dependencies sections, and EA PR #221 (FR-044) points back at quoin FR-149 and FR-142, with matching roles: EA produces the report through `producer-execute`, quoin reads it through the adapter and joins it. No cycle is introduced (FR-149 -> FR-142 -> FR-144).

## Verdict

PASS with one low finding.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | FR-149 and FR-142 target `ix://agent-ix/engineering-assurance/FR-044`, which exists only on the open EA PR #221, and EA FR-044 targets quoin FR-149, which exists only on this PR. Either merging alone leaves a dangling cross-repo edge. Fix: merge PR #846 and EA PR #221 together. | spec/functional/FR-149-grype-advisory-report-adapter.md:16-17, 87 |
