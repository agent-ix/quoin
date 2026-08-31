---
id: SR-085
title: "Dependency review of issue 152 graph-analysis requirements"
type: SpecReview
analysis: dependency
scope: "US-018, FR-062, TM-001 TC-1249..TC-1260"
review_set: all
---

# Dependency review of issue 152 graph-analysis requirements

## Summary

The slice has an acyclic enablement chain: the retained Quoin evidence/auditor contracts and
quire-rs's versioned assurance export precede the user-visible graph views. Portfolio reporting in
issue #281 is downstream and may consume the reports without changing their semantics.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The prerequisite graph is explicit, acyclic, and keeps export enablement separate from Quoin's graph-view feature work. | FR-030; FR-032; quire-rs FR-067/FR-068; FR-062 |

## Classification and order

| Requirement | Class | Rationale |
| --- | --- | --- |
| quire-rs FR-067 | Enablement | Defines the accepted versioned assurance-export envelope. |
| quire-rs FR-068 | Enablement | Supplies authoritative artifacts, obligations, symbols, and relationships. |
| FR-030 | Enablement | Supplies retained bindings and reaffirmation history. |
| FR-032 | Enablement | Supplies auditor verdicts without reinterpretation. |
| FR-062 | Feature | Exposes fan-out, change-impact, and churn reports to the user. |

Topological order: quire-rs FR-067/FR-068 plus already-landed FR-030/FR-032, then FR-062, then
issue #281 portfolio consumption. No cycle is present.
