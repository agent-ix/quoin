---
id: SR-20261123163422
title: "EARS conformance review of PLAT-1806 evidence record exit status"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-030-evidence-store.md; spec/functional/FR-033-evidence-format-adapters.md (PR #1025, ticket PLAT-1806)"
review_set: subset
---

# EARS conformance review of PLAT-1806 evidence record exit status

## Summary

Ticket: PLAT-1806. `quire validate --scope . spec/functional/FR-030-evidence-store.md spec/functional/FR-033-evidence-format-adapters.md --summary` reports 2/2 documents grammar-clean with 0 grammar findings. The new FR-030 SHALL states a trigger ("when a trace id is unmatched", "when the run has at least one entry and no entry carries a trace id") and an observable response (non-zero exit after publication).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

The text conforms to the EARS grammar. Its semantic gaps (the exit value and the adapter scope) are recorded in the base review SR-20261123163420.

## Examined

The FR-030 SHALL on `record` exit status (examined).
