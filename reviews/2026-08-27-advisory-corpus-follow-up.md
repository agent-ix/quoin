---
id: SR-033
title: "Follow-up — repair the 108 confirmed advisory corpus defects"
type: SpecReview
analysis: gap-analysis
scope: "bench/advisory-adjudication-v1.json and qa-corpus fixture repairs"
review_set: subset
---

# Follow-up — repair the 108 confirmed advisory corpus defects

## Summary

The `finding.precision.advisory-v1` adjudication at
`bench/advisory-adjudication-v1.json` confirms all **108** previously
unclassified findings against the pinned source population. They are correct
producer observations with a controlled-corpus repair target.

## Findings

| ID      | Severity | Summary                                                                        | Refs                                  |
| ------- | -------- | ------------------------------------------------------------------------------ | ------------------------------------- |
| FND-001 | medium   | 104 `catch-all-universal` rows require a specific fixture property shape       | `bench/advisory-adjudication-v1.json` |
| FND-002 | medium   | 4 `archetype-matches-nothing` rows require matching TestMatrix fixture records | `bench/advisory-adjudication-v1.json` |

## Detail

| Family                      | Rows | Corpus repair                                                                                                                                                                                           |
| --------------------------- | ---: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `catch-all-universal`       |  104 | Give the shared extractable fixture criterion a declared specific property shape, preserving each case's seeded defect and healthy differential.                                                        |
| `archetype-matches-nothing` |    4 | Add the smallest language-neutral `TestMatrix` document needed by the `test-case` declaration in the two affected case/control pairs, preserving their verification-method and empty-row differentials. |

The exact rows, source envelopes, reviewer, rationale, rubric version,
disagreement list, and this follow-up path are retained per finding in the JSON
artifact. Its population identity is
`sha256:dd9533344090240acb0b0ea1aa80b04a37a599040a5f584425bb74696fc2ac2c`.

This follow-up does not authorize a bulk fixture rewrite inside #258. Apply the
normal qa-corpus banking and differential gates when repairing the cases, then
collect a new benchmark population rather than rewriting the retained ruling
record.
