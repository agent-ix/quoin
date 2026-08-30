---
id: SR-053
title: "Scope-boundary review of issue 288 semantic type-fit audit requirements"
type: SpecReview
analysis: scope-boundary
scope: "US-014, FR-051..FR-055, NFR-015..NFR-016"
review_set: all
---

# Scope-boundary review of issue 288 semantic type-fit audit requirements

## Summary

Quoin owns the census implementation and retained audit. Module repositories own their declarations
and examples; Quire owns Markdown parsing and semantic extraction; `filament-core-data` owns shared
core-contract candidates; the architecture record owns plane and authority terminology. The audit
observes those authorities and creates no replacement contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-044 | low | Positive ownership, exclusions, and cross-boundary reconciliation are explicit; no compiler or migration authority is allocated to the audit. | FR-051; FR-055; NFR-016 |

## Boundary allocation

| Boundary | Audit responsibility | Excluded responsibility |
| --- | --- | --- |
| Default-module manifest | Enumerate and identify entries | Change pins or reconcile installations |
| Module repository | Read declarations and Markdown | Edit manifest, schema, skeleton, or source |
| Quire | Invoke/record parsing evidence | Change parser, validator, extractor, or splicer |
| Core data | Reconcile census overlap | Mint a shadow shared contract |
| Consumer repositories | Assess likely impact | Change generated packages, APIs, databases, CLIs, or UIs |
| Campaign governance | Recommend ticket boundaries and gates | Authorize breaking promotion |
