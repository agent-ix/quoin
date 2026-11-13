---
id: SR-20261113052923
title: "Failure-domain review of PLAT-1601 OSV-Scanner adapter and advisory join"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; PR #881; spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-149-osv-scanner-advisory-report-adapter.md"
review_set: all
---

# Failure-domain review of PLAT-1601 OSV-Scanner adapter and advisory join

## Summary

Reviewed the OSV report boundary, identity and match rules, failure refusal cases, and read-only join behavior. The adapter and join are explicitly pure over retained report text, use purl identity where available, and leave unmatched items visible. A malformed package identity can still be accepted without a defined refusal or safe representation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-149 defines the fallback path from `package.name` and `package.version`, but its refusal rules do not require those fields or a valid `package` object when a vulnerability is present. A report with a vulnerability id but missing package identity can be recorded with an unusable or ambiguous path, and the spec does not say whether to refuse it or preserve it as unmatched. Define the required package identity fields and the failure behavior. | spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:77; spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:100; spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:114 |

## Findings rationale

The remaining failure-domain checks are clean for this change: the adapter opens only the input, does not execute tools or network calls, and the join does not infer identities across purl types. No graph traversal or callback extension point is introduced.
