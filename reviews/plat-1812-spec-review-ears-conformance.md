---
id: SR-20261123190540
title: "EARS review of SOUP promotion keeping SBOM supplier and licence"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-141-promote-sbom-components-to-soup-records.md"
review_set: subset
---

# EARS review of SOUP promotion keeping SBOM supplier and licence

## Summary

Ticket: PLAT-1812 (PR agent-ix/quoin#1030). EARS conformance of the twelve new and edited FR-141 Behavior
statements. Most are well-formed ubiquitous, unwanted-behaviour or state-driven statements in the file's existing
style; two are not atomic.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "If a supplier or licence text contains a control character, then the command SHALL omit that field ... and list the component and field in the report" carries two responses; the same file splits such pairs into two statements (module not installed, `--discharges` unmatched). | FR-141 Behavior (control character in supplier or licence) |
| FND-002 | low | The two existing-record difference statements each join two trigger conditions with "or" under one "Where" clause ("differs ..., or the SBOM states one and the record states none"), making each statement compound and hiding the uncovered converse case (see the failure-domain review). | FR-141 Behavior (existing record supplier and licence differences) |

## Verdict

EARS defects are low only. No new statement lacks a SHALL, a subject, or a testable response.

## Examined

FR-141 Behavior statements added or edited by this change: supplier written / absent, licence exactly one / none /
several (two), control character, no `license_fact_ref`, no literal `unknown`, two difference statements,
`license_fact_ref` suppresses licence difference, report contents.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 19245c58 (round 1) |
| FND-002 | fixed | Fix commit 19245c58 (round 1) |
