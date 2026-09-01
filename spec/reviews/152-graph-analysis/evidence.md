---
id: SR-086
title: "Evidence review of issue 152 graph-analysis requirements"
type: SpecReview
analysis: evidence
scope: "FR-062, TM-001 TC-1249..TC-1260"
review_set: all
---

# Evidence review of issue 152 graph-analysis requirements

## Summary

The catalog advisor found an applicable recommendation for every FR-062 obligation. The authored
Test/Inspection classes and planned Unit, Property, Integration, and Static evidence cover the
recommended behaviors; the sole class mismatch is a justified static-boundary inspection.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The advisor recommends Test for AC-11 from its example shape, but the criterion is a dependency-boundary inspection and TM-001 correctly plans a Static check; retain the authored Inspection method. | FR-062-AC-11; TC-1259 |

## Advisor conclusion

AC-1/3/6/8/10 receive property, metamorphic, or deterministic-output recommendations and have
Property rows where permutation or deduplication is load-bearing. AC-2/4/5/7/9/12 have executable
negative, invariant, or regression oracles. AC-11's static dependency prohibition is more directly
verified by import/dependency inspection than by a behavioral example, so the human judgment keeps
Inspection. No obligation is inconclusive or uncatalogued.
