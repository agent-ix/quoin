---
id: SR-16395
title: "EARS conformance review of PLAT-1021 graded-corpus collection producer"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-108-measurement-verdict-checker.md (FR-108 statement, FR-108-AC-15..AC-21) -- PR #779 diff only"
review_set: subset
---

## Summary

Ticket: PLAT-1021. The diff adds no requirement statement; it adds seven ACs
and a design section under FR-108's existing statement. The statement is a
conformant ubiquitous SHALL about `quoin measurement verify`. The new ACs
specify a producer, which that statement does not cover.

## Scope examined

- FR-108 statement (examined)
- FR-108-AC-15 .. AC-21 (examined)

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | FR-108's only SHALL is the checker deciding a verdict "independent of the producer"; AC-15..AC-21 specify a producer (quoin-jev graded_corpus_collection and the live harness write) that no SHALL statement covers, so the ACs exceed their FR statement (AC-10 set the precedent) | FR-108; FR-108-AC-15..AC-21 |

## Dispositions

| FND | Outcome | Reason |
| --- | ------- | ------ |
| FND-001 | fixed | Round 1: FR-108 Description adds "quoin-jev SHALL provide a pure function that builds the complete graded-corpus MeasurementCollection ...", covering the producer ACs; AC-21..AC-23 are the harness caller's criteria under that statement. |
