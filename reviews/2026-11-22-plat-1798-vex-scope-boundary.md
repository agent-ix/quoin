---
id: SR-202611220107
title: "Scope-boundary review of PLAT-1798 FR-144 VEX population amendment"
type: SpecReview
analysis: scope-boundary
scope: "PR #1008 at 7121d53c; FR-144 report boundary and VEX input ownership"
review_set: all
---

## Summary

FR-144 owns presentation of VEX statements in the SOUP report; the person-authored statements and their validity remain owned by the upstream SOUP record contract. No new report-side authority to create or interpret VEX data is introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional scope-boundary issue found beyond FR-144's conflict with the upstream FR-022 input contract recorded in the integrity review. | - |

## Boundary record

The report consumes statements on `soup_record` inputs and advisory matches from FR-142; it may display source facts but cannot override input validity or advisory disposition. FR-156 is a separate export consumer. Measured with quoin 0.36.8 and quire 0.52.0 (engine 0.62.2).
