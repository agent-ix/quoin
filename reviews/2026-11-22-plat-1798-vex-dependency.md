---
id: SR-202611220104
title: "Dependency review of PLAT-1798 FR-144 VEX population amendment"
type: SpecReview
analysis: dependency
scope: "PR #1008 at 7121d53c; FR-144, upstream FR-142 and spec-objects-security FR-022, downstream FR-156"
review_set: all
---

## Summary

The change remains a report behavior over existing SOUP records and joined advisories. Existing dependencies cover its input sources; no new enablement requirement or ordering edge is needed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional dependency or sequencing issue found; the separate source-contract conflict is recorded in the integrity review. | - |

## Dependency disposition

FR-142 supplies joined advisories, spec-objects-security FR-022 owns valid VEX statement fields and uniqueness, FR-144 consumes both for the report, and FR-156 exports stored statements. The implementation order is unchanged; resolving the invalid duplicate-input premise belongs with FR-022 if duplicate ids are to become valid. Measured with quoin 0.36.8 and quire 0.52.0 (engine 0.62.2).
