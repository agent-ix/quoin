---
id: SR-087
title: "Risk and complexity review of issue 152 graph-analysis requirements"
type: SpecReview
analysis: risk-complexity
scope: "US-018, FR-062, TM-001 TC-1249..TC-1260"
review_set: all
---

# Risk and complexity review of issue 152 graph-analysis requirements

## Summary

FR-062 has medium technical risk and medium pre-landing volatility because it joins a new external
export contract with retained evidence and performs cyclic graph closure. Contract, property,
boundary, and regression tests are named mitigations; no high unmitigated risk remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | All identified contract, topology, completeness, and accidental-verdict risks have named executable mitigations in TM-001. | FR-062; TC-1249..TC-1260 |

## Risk register

| Req | Tech Risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| FR-062 | Medium | Medium | New quire-rs export contract, cyclic/shared graph topology, three-store join, deterministic rendering | Vendor and validate the exact schema; property-test closure/deduplication/permutations; static-test boundaries; preserve golden regressions. |

## Top hazards

1. Export-schema or module-premise drift could admit the wrong graph; TC-1256/TC-1257 fail closed.
2. Missing inputs could appear as a healthy zero; TC-1250/TC-1252/TC-1257 preserve named gaps.
3. Closure could become nondeterministic or superlinearly repeat work; TC-1251/TC-1258 exercise
   cycles, shared dependents, shortest-path selection, and input permutations.
4. Change exposure could overwrite an auditor verdict; TC-1253 keeps the facts separate.

Failure-domain cross-check: SR-083 reports no open gap.
