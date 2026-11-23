---
id: SR-1854003
title: "EARS conformance review of FR-029 dropped-declaration refusal (PLAT-1854)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-029-consume-the-quire-json-contract.md"
review_set: subset
---

# EARS conformance review of FR-029 dropped-declaration refusal (PLAT-1854)

## Summary

Ticket: PLAT-1854. PR agent-ix/quoin#1036, head 3a65d0af. This review checks the three new FR-029 Behavior bullets against EARS requirement grammar.

- Bullet 1 is a well-formed event-driven requirement: "When the linked engine reports …, `quoin-quire` SHALL return …".
- Bullet 2 is ubiquitous.
- Bullet 3 states its trigger after the response: "SHALL refuse … when any declaration was dropped".

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Behavior bullets 2 and 3 are compound. Bullet 2 bundles the class mapping, the envelope content and a design note about an exhaustive match. Bullet 3 bundles the refusal, the enumerated command list and an unmarked rationale sentence ("A drop never degrades …") that states a requirement with no SHALL. Split each into one SHALL per obligation, phrase bullet 3 as an unwanted-behaviour "If any declaration was dropped, then each command SHALL …", and give the QQ-1020 non-degradation its own SHALL. | spec/functional/FR-029-consume-the-quire-json-contract.md:54-67 | wrong-requirement |

## Verdict

One low finding. The grammar does not block the PR. The merge decision rests on SR-1854002.

## Examined

FR-029 Behavior bullets 1 to 4 (the first is unchanged) and FR-029-AC-14..18 wording.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | dec7dd14: Behavior is split into one SHALL per obligation. The refusal and the QQ-1020 non-degradation are each unwanted-behaviour "If any declaration was dropped, then … SHALL" requirements, and the rationale sentence is gone. |
