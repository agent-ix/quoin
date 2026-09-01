---
id: SR-096
title: "Scope-boundary review of Quoin change-assurance contracts"
type: SpecReview
analysis: scope-boundary
scope: "US-017 and FR-063..FR-065"
review_set: all
---

# SR-096: Scope-boundary review of Quoin change-assurance contracts

## Summary

Quoin owns integrity parsing, storage, and deterministic verification over
retained inputs. Proof producers, Git, networks, identity, ix-flow decisions,
FR-032 auditing, and policy promotion remain outside this ticket.

## Findings

| ID      | Severity | Summary                                                                                                                                                | Refs                                           |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------- |
| FND-001 | low      | No responsibility leak remains: every external system is consumed as retained evidence and every execution or identity claim is explicitly prohibited. | FR-063-CON-1/3; FR-064-CON-1/3; FR-065-CON-1/3 |

## Allocation

FR-063 accepts source and workflow references without discovering them. FR-064
transcribes producer output without running it. FR-065 consumes ix-flow and
FR-032 results without replacing either source's verdict.
