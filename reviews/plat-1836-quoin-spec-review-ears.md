---
id: SR-2026112016223475846355011
title: "PLAT-1836 quoin PR 1032 spec-review EARS"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-160-per-class-evidence-floors.md, spec/functional/FR-161-per-class-obligations-report.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-160"
    type: references
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
---

# PLAT-1836 quoin PR 1032 spec-review EARS

## Summary

Ticket: PLAT-1836. EARS conformance of the requirement text this PR edits. The FR-160 and FR-161 statements are unchanged and remain conformant ("`quoin evidence audit` and `quoin matrix` SHALL, when given `--class-policy <file>`, apply ..."). The PR adds a new optional, event-driven behaviour (the gate) that is not expressed as a requirement statement, and its criteria leave the exit status unpinned.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The gate is a new behaviour with its own trigger (`gate: true`) and response (exit `Partial`), but appears only as Description prose without a SHALL clause in either requirement's statement. EARS optional-feature form would be: "WHERE the class policy sets `gate: true`, `quoin evidence audit` SHALL exit `Partial` WHEN a reported finding carries a `class` member" (FR-160) and the matching clause for `quoin class obligations` with an `unmet` row (FR-161). | spec/functional/FR-160-per-class-evidence-floors.md:106-117; spec/functional/FR-161-per-class-obligations-report.md:83-87 |
| FND-002 | low | FR-160-AC-16 and FR-161-AC-13 say "exits non-zero" while the prose specifies `Partial` (exit 1, payload still emitted). A refused policy or an internal error (exit 2 or 3, no report) satisfies the criteria as written, so they do not pin that the gate changes only the exit status. State "exits `Partial` (1) with the same report as without `gate`". | spec/functional/FR-160-per-class-evidence-floors.md:143; spec/functional/FR-161-per-class-obligations-report.md:105 |

## Verdict

APPROVE WITH LOW FINDINGS. Statements conform; the new gate behaviour should be stated as an optional-feature requirement clause and its criteria should name the exit status.

## Examined

- FR-160 statement (examined)
- FR-161 statement (examined)
- FR-160-AC-14, FR-160-AC-15, FR-160-AC-16 (examined)
- FR-161-AC-10, FR-161-AC-11, FR-161-AC-12, FR-161-AC-13 (examined)

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | a102c716: FR-160 adds "Quoin SHALL exit `quoin evidence audit` with `Partial` (exit 1) when the policy sets `gate: true` and at least one finding carries a `class` member" and a SHALL NOT for the ungated case; FR-161 "Unmet activities can gate" now states "`quoin class obligations` SHALL exit `Partial` (exit 1) when any row is `unmet`, and SHALL NOT change its report". |
| FND-002 | fixed | a102c716: FR-160-AC-16 and FR-161-AC-13 now say "exits `Partial` (exit 1) ... with the report unchanged"; the tests assert `Some(1)` and byte-identical output. |
