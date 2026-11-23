---
id: SR-20261123202513
title: "EARS review of release snapshot risk-completeness and anomaly-gate open findings"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-166-release-snapshot-command.md, spec/functional/FR-167-release-snapshot-format.md"
review_set: subset
---

## Summary

Ticket: PLAT-1808 (PR agent-ix/quoin#1033). EARS conformance of the four new FR-166 behaviors and the five new
FR-167 behaviors. Ubiquitous and unwanted-behavior (`If ... then ... SHALL`) forms are used correctly and each
statement has one actor. Two statements carry a rationale clause inside the requirement.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | The FR-166 statements "If a gate failure exists, then the command SHALL still write the snapshot, because the failure is an open finding ..." and the risk-summary refusal "..., because a finding cannot be omitted unannounced" put the rationale inside the EARS statement; move each `because` clause to the Description. | FR-166 |

## Verdict

Conformant apart from one low wording finding.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Both `because` clauses moved to the Description; the two statements are now plain unwanted-behavior EARS. |
