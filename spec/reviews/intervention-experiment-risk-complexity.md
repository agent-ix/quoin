---
id: SR-064
title: "Risk and complexity review of intervention-experiment evidence"
type: SpecReview
analysis: risk-complexity
scope: "US-015; FR-056; FR-057; TC-1195..TC-1216"
review_set: all
---

## Summary

FR-056 has high semantic risk and medium volatility because multi-arm identity and
causal preconditions define the trust boundary. FR-057 has medium technical risk
and low-to-medium volatility around atomic storage, verification, and deterministic
projection; the matrix emphasizes these boundaries at P0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Mitigated: multi-arm attribution and causal validity are explicit invariants with property and boundary tests. | FR-056-AC-4; FR-056-AC-5; FR-056-AC-8; TC-1198; TC-1199; TC-1202 |
| FND-002 | medium | Mitigated: the intake trust boundary verifies retained raw bytes before one atomic record write and returns stable refusal codes. | FR-057-AC-1; FR-057-AC-11; TC-1204; TC-1216 |
