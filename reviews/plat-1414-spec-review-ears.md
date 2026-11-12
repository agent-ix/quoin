---
id: SR-20261112024904
title: "EARS conformance review of quoin PR 828 (FR-136)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin@b92fb76; spec/functional/FR-136-anomaly-record-mapping.md"
review_set: subset
---

# EARS conformance review of quoin PR 828

## Summary

Ticket: PLAT-1414. Classified the Description and the nine Behavior statements of FR-136. The
`quire validate` grammar pass gives no EARS warning on FR-136. Six of the Behavior statements are
ubiquitous and three are unwanted-behaviour (`If ... then`). All of them name the subject `Quoin`
and a concrete response.

## Verdict

**PASS WITH FINDINGS, one low.** Two Behavior bullets each pack two independent obligations
under separate `SHALL`s.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Two bullets are non-singular. Lines 41-43 combine the row-to-entry mapping and Target resolution, two obligations with different failure modes. Lines 44-46 combine keep-entry, report-diagnostic and do-not-substitute. Split each into one statement per `SHALL`, so that each AC traces to one statement | spec/functional/FR-136-anomaly-record-mapping.md:41-46 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-136, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "Description and Behavior bullets lines 21-54; ubiquitous x6, unwanted (If/then) x3"}
```
