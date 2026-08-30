---
id: SR-042
title: "Risk and complexity review of issue 289 semantic-module architecture requirements"
type: SpecReview
analysis: risk-complexity
scope: "US-013, FR-046..FR-050, NFR-013..NFR-014"
review_set: all
---

# Risk and complexity review of issue 289 semantic-module architecture requirements

## Summary

The change is low implementation complexity but high architectural blast radius because later
compiler, module, and consumer work will cite it. The requirements therefore make provenance,
provisional status, compatibility, and pre-merge maintainer review first-class gates.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-033 | low | Residual risk is governance drift rather than implementation uncertainty and is bounded by the external-decision ledger, tests, and maintainer gate. | FR-050-AC-6; NFR-013; NFR-014 |

## Risk register

| Risk | Likelihood | Impact | Control |
| --- | --- | --- | --- |
| Provisional TypeSpec work is read as accepted | medium | high | FR-048-AC-3 plus explicit provisional labeling test. |
| New compiler ownership absorbs Quire or Quoin responsibilities | medium | high | FR-047 positive and negative ownership assertions. |
| Generated packages close the runtime module ecosystem | medium | high | FR-049 dynamic/static compatibility assertions. |
| Old Quire rendering language is revived | low | high | FR-050 decision-by-decision disposition. |
| Active feature work is disrupted | low | high | NFR-014 path guard, existing gates, and maintainer review before merge. |
