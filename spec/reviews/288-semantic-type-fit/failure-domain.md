---
id: SR-048
title: "Failure-domain review of issue 288 semantic type-fit audit requirements"
type: SpecReview
analysis: failure-domain
scope: "US-014, FR-051..FR-055, NFR-015..NFR-016"
review_set: all
---

# Failure-domain review of issue 288 semantic type-fit audit requirements

## Summary

The review exercised missing repositories, repointed refs, dirty worktrees, duplicate declarations,
unparseable documents, absent schemas and instances, placeholder contracts, stale external evidence,
partial output, and accidental source mutation. Each state is retained rather than subtracted from a
denominator or normalized into a clean verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-039 | low | No unhandled identity, enumeration, parsing, scoring, publication, or mutation failure domain remains. | FR-051-AC-5; FR-052-AC-1..AC-7; FR-054-AC-5; FR-055-AC-5; NFR-016 |

## Failure inventory

| Failure shape | Required response |
| --- | --- |
| Requested ref and inspected bytes disagree | Retain a `provenance-conflict` and block a clean verdict. |
| A module or document cannot be loaded | Keep it in the relevant denominator with an unresolved or parse-error state. |
| A schema accepts any object | Mark the schema as placeholder; do not infer round-trip or generation suitability. |
| Two modules declare the same name | Preserve both qualified declarations and record incompatible definitions. |
| A canonical artifact is missing or stale | Reject the summary rather than publish partial results as complete. |
| The corpus moves before signoff | Mark the review stale and require a fresh census. |
| A recommendation crosses a breaking boundary | Stop at the named human gate; do not activate the change. |
