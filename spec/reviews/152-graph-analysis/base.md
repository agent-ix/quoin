---
id: SR-082
title: "Base review of issue 152 graph-analysis requirements"
type: SpecReview
analysis: base
scope: "US-018, FR-062, TM-001 TC-1249..TC-1260"
review_set: all
---

# Base review of issue 152 graph-analysis requirements

## Summary

The issue #152 slice defines one read-only graph-report contract over a validated Quire assurance
export, Quoin's retained evidence store, and existing auditor verdicts. The identifiers, command
surface, explicit export/premise/audit file inputs, input/output states, deterministic ordering,
and failure cases are specific and all twelve acceptance criteria have planned Test Matrix rows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved during review: the original text did not say how the CLI obtained the accepted export, premises, or precomputed verdicts. Every subcommand now requires `--export`, `--premises`, and `--audit`, while `--repo` is limited to retained Quoin state. | FR-062; TC-1257 |

## Coverage conclusion

US-018 supplies three testable Given/When/Then examples and traces to StR-004. FR-062 links the
story and every upstream contract, defines inputs, outputs, behavior, constraints, and twelve
criteria. TM-001 maps TC-1249..TC-1260 one-to-one across those criteria, including malformed,
missing, unreadable, rejected-premise, mismatched-audit, incomplete, empty, cyclic, duplicate,
permutation, static-boundary, and non-regression cases.
