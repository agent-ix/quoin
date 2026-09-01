---
id: SR-095
title: "Risk and complexity review of Quoin change-assurance contracts"
type: SpecReview
analysis: risk-complexity
scope: "FR-063..FR-065"
review_set: all
---

# SR-095: Risk and complexity review of Quoin change-assurance contracts

## Summary

The lane is high-integrity but bounded: RFC 8785 compatibility, raw JSON edge
cases, crash-safe paired persistence, lineage, and three-state aggregation are
the volatile implementation surfaces. The specifications assign direct tests
and properties to each surface.

## Findings

| ID      | Severity | Summary                                                                                                                                                       | Refs                                |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FND-001 | medium   | Canonicalization and raw JSON parsing are the highest implementation risk; conformance vectors and malformed-input properties must precede store integration. | FR-063-AC-5..AC-7; TC-1265..TC-1267 |
| FND-002 | medium   | Crash-atomic two-file persistence requires fault injection around staging, close, and rename rather than happy-path integration alone.                        | FR-064-AC-6; TC-1277                |
| FND-003 | medium   | Outcome precedence and input permutations are combinatorial and should be property-tested as pure functions before CLI wiring.                                | FR-065-AC-2..AC-5; TC-1282..TC-1285 |

## Sequencing consequence

Implement pure canonicalization and evaluators first, then persistence, then
adapters. This isolates high-risk semantics from filesystem and CLI effects.
