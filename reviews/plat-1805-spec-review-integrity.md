---
id: SR-17093
title: "integrity review of FR-211 and the FR-046/FR-208 reversal"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md, spec/functional/FR-046-clause-discharge.md, spec/functional/FR-208-module-specific-discharge-approval.md, spec/functional/FR-173-facts-from-mappings.md, spec/functional/FR-133-discharge-facts-in-the-evidence-store.md"
review_set: subset
---

## Summary

Ticket: PLAT-1805. Consistency check of PR agent-ix/quoin#1026: does the reversal in
FR-046/FR-208 (approval references go from opaque to verified for approval-class
clauses; `evidenceRefs` go from free text to resolved store paths) leave contradictory
text anywhere in `spec/`. Searched every requirement and test case naming
`evidenceRefs`, `approvalRef`, approval references or authentication disclaimers.
FR-173 is left contradicting FR-211, and FR-211 and FR-208 each contradict themselves.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-173 emits direct facts whose `evidenceRefs` are the `ix://` ids of controls, and FR-173-AC-7/AC-12 require `record-discharge` to accept them; FR-211 refuses every non-`evidence://` ref as `unresolved`. FR-173-AC-11 also keeps a `scope_exclusion` approval reference opaque, which FR-211 refuses on an approval-class clause | FR-173-AC-7, FR-173-AC-11, FR-173-AC-12, FR-211 | wrong-requirement |
| FND-002 | high | FR-211-AC-3 refuses a discharge-fact or trust-decision path with `kind-not-expected`, but the Behavior says any family outside the list (`discharge`, `trust`) is `unresolved`; a test written to either one fails against the other | FR-211-AC-3, FR-211 | wrong-requirement |
| FND-003 | medium | FR-208 Description and Outputs still say approval-required clauses are direct "only when the required approval reference is present", and FR-208 bullet 3/AC-2 limit verification to roles "that require a named human approver", a predicate nothing defines, while FR-211 verifies every clause with non-empty `approvalRoles` | FR-208-AC-2, FR-208, FR-211 | wrong-requirement |
| FND-004 | low | The FR-046 constraint attaches "a direct fact's `evidenceRefs` are resolved (FR-211)" to the approval-class sentence, so it reads as approval-class only, but FR-211 resolves refs for every direct fact; the same section still says the implementation "uses only caller-provided data" although admission now reads the store and runs engineering-assurance | FR-046, FR-211 | wrong-requirement |

## Verdict

FAIL. Not mergeable until FND-001 and FND-002 are fixed. FND-001 needs either FR-173
amended to emit `evidence://` refs (controls are not retained records, so this is a
real design choice about whether a control mapping can discharge a clause), or FR-211
to admit control ids as a ref kind.

Clean (examined, no contradiction): FR-135 and FR-142 only copy `approvalRef` and
evidence references through unchanged; TC-626 and TC-003 cover clauses with empty
`approvalRoles`, which keep FR-046 behaviour; FR-208-CON-2 and FR-211-CON-1 agree that
no risk score, threshold or acceptability is computed; index.md and spec.md entries
match the new file's id and title.
