---
id: SR-20261115183520
title: "EARS conformance — PLAT-1760 FR-115 typed stale-evidence cause"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md; PLAT-1760"
review_set: subset
---

## Summary

Ticket: PLAT-1760. EARS reading of the two statements PR #931 adds to or edits
in FR-115's Outputs section (lines 202-208 and 216-228). The engine check
(`quire validate --summary`) could not run because approval was required, so
this reading is manual. The FR-115 Description statement and the Behavior
SHALL list are unchanged.

## Verdict

**CONDITIONAL**, with one low finding. The edited `findings` bullet (line 202) is
descriptive output-shape prose, like the bullets around it, and needs no
SHALL. The new normative content has no SHALL statement anywhere. The ACs carry
it, so nothing is lost at the test level.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new obligation, "A `stale-evidence` `Finding` therefore carries an optional typed `cause`, set by the auditor's two existing producers ... It is **omitted** (never `null`) on every other `FindingKind`", has no system subject and no SHALL, and nothing in Behavior matches it. When FR-153 analysis evidence was added, it got Behavior SHALL statements. Something like "When the auditor raises `stale-evidence` because a bound suite has no recorded run, it SHALL set `cause: missing-run`" would make this traceable in the same way. | spec/functional/FR-115-evidence-backed-test-matrix.md:216-228 |

## Dispositions

Round 1, fix commit 0883d0c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-032 and FR-115 now state system-subject SHALL behavior for producing, copying and refusing typed causes. |
