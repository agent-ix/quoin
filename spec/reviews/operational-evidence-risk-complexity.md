---
id: SR-072
title: "Risk and complexity review of operational evidence"
type: SpecReview
analysis: risk-complexity
scope: "US-016; FR-059; FR-060; TC-1223..TC-1243"
review_set: all
---

## Summary

FR-059 has high semantic risk and medium volatility around temporal and cross-record
invariants. FR-060 has high decision risk and medium technical complexity because
its discharge and report projections influence assurance decisions; both remain P0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Mitigated: clock-state derivation and discharge matching are explicit invariants with property evidence. | FR-059-AC-6; FR-060-AC-7; TC-1228; TC-1238 |
| FND-002 | high | Mitigated: adverse capability/exercise states and raw-evidence mismatch cannot become affirmative evidence. | FR-060-AC-2; FR-060-AC-8; TC-1233; TC-1239 |
