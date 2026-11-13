---
id: SR-20261113071109
title: "EARS conformance review of quoin PR 854 (FR-136 fixing change)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin@af5b091b; spec/functional/FR-136-anomaly-record-mapping.md"
review_set: subset
---

# EARS conformance review of quoin PR 854

## Summary

Ticket: PLAT-1527. Classified the four new Behavior statements at lines 87-96. Three are
ubiquitous with the subject `Quoin`. One is a definition whose subject is not the system.
`quire validate` reports no EARS warning on FR-136.

## Verdict

**PASS WITH FINDINGS, two low.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "The fixing change of an anomaly resolved as `fixed` ... SHALL be the first `statusHistory` entry ..." has the fixing change as its subject, not the system, so no actor carries the obligation. Write it as "Quoin SHALL take as the fixing change ..." | spec/functional/FR-136-anomaly-record-mapping.md:87-89 |
| FND-002 | low | "Quoin SHALL expose ... `fixingChange` ..., absent when no such entry exists or when `resolved` is only pending" puts an unwanted-behaviour condition inside a ubiquitous statement. Split it into "Quoin SHALL expose ..." and "If no such entry exists, or `resolved` is only pending, then Quoin SHALL omit `fixingChange`" | spec/functional/FR-136-anomaly-record-mapping.md:90-92 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-136, path: spec/functional/FR-136-anomaly-record-mapping.md, role: examined, excerpt: "Behavior lines 87-96: fixing-change definition (non-system subject), expose fixingChange (ubiquitous with embedded condition), SHALL NOT read a hash (ubiquitous negative), SHALL NOT check agreement (ubiquitous negative)"}
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
