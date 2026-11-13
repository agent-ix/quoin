---
id: SR-20261113052927
title: "Risk and complexity review of PLAT-1601 OSV-Scanner adapter and advisory join"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/quoin; PR #881; FR-142, FR-144, FR-149 and affected NFR-030"
review_set: all
---

# Risk and complexity review of PLAT-1601 OSV-Scanner adapter and advisory join

## Summary

The new scanner report contract is a high technical-risk and high-volatility dependency because the adapter consumes an external JSON format and its package-to-purl behavior is partly unverified. The research table marks unresolved assumptions and AC-10 calls for captured fixtures; the ticket's OS-package coverage gap is separately recorded in the base review. No additional defect was found in this risk lens.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Top hazards

1. OSV-Scanner report shape and package purl behavior can change; the research table marks purl and severity details unverified. Retained report fixtures and adapter tests are the stated mitigation.
2. Offline database coverage and freshness are external to the report envelope; the adapter explicitly records the database as not recorded, leaving recency to the person interpreting the scan timestamp and external run context.
3. OS-package advisory coverage is unverified and excluded from the assessed purl table; see the high-severity base finding for the outstanding ticket decision.
