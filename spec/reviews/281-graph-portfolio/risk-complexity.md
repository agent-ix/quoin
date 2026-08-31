---
id: SR-103
title: "Risk and complexity review of Quoin #281 graph portfolio requirements"
type: SpecReview
analysis: risk-complexity
scope: "StR-007, FR-066, FR-067"
review_set: all
relationships:
  - target: "ix://agent-ix/quoin/FR-066"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-067"
    type: reviews
---
# Risk and complexity review of Quoin #281 graph portfolio requirements

## Summary

Contract integrity and compatibility are the dominant technical risks. The
requirements are low-volatility inside Quoin, while the external graph-quality
producer contract is medium-volatility until its implementation ships.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | All high-risk or volatile surfaces have named mitigations | FR-066, FR-067 |

## Risk Register

| Requirement | Tech Risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| StR-007 | medium | low | Evidence authority can be blurred by convenience aggregation | Negative tests for execution, synthesis, and aggregate verdicts |
| FR-066 | high | medium | External closed contracts, canonical identity, digest integrity, bijective normalization | Schema mutation tests, byte fixtures, property tests, fail-closed adapter registry |
| FR-067 | high | low | Comparison across unlike populations can produce plausible misinformation | Premise-by-premise compatibility properties and no-delta assertions |

## Top Hazards

1. FR-066 accepts a mutated record or attachment under the original identity.
2. FR-067 compares collections after one population premise changes.
3. FR-067 hides readable history because one collection is corrupt.

## Failure-Domain Cross-Check

SR-099 records no open failure-domain gap. The mitigations above map to
TC-1296..TC-1301 and TC-1306..TC-1313.
