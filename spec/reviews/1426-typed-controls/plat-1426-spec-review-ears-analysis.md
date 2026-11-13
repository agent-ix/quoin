---
id: SR-20261113140424
title: "EARS conformance review of typed control and clause-set output (FR-171 to FR-174, NFR-038)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #871 (git diff origin/main...plat-1426-typed-controls-spec at 20e50a4a): spec/functional/FR-171, FR-172, FR-173, FR-174, spec/non-functional/NFR-038"
---

# EARS conformance review of typed control and clause-set output (FR-171 to FR-174, NFR-038)

## Summary

Ticket: PLAT-1426. Every Behavior bullet and every Statement SHALL was checked against the EARS patterns:
ubiquitous, event-driven, unwanted-behaviour "If ... then" and state-driven. Most bullets are atomic and
conform, and the author has already split the "when no owner is known" and "set not installed" cases into
separate bullets.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | One bullet holds three requirements: the `delegated` disposition is emitted for an excluded binding clause, its rationale is composed from rationale and owner, and its `approvalRef` comes from `--approval-ref`. Split the bullet. | spec/functional/FR-173-facts-from-mappings.md:43 |
| FND-002 | low | Three unwanted-behaviour triggers share one bullet: a missing or incomplete attestation, a clause that is both mapped and excluded, and an invalid corpus. Write each as its own "If ..., then the command SHALL exit non-zero and emit nothing". | spec/functional/FR-173-facts-from-mappings.md:46 |
| FND-003 | low | Normative SHALLs sit in Description prose, not in Behavior: FR-172's "The skill SHALL NOT supply its text", and FR-174's "The report SHALL show each clause ...". Move them to Behavior. | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:29; spec/functional/FR-174-per-clause-compliance-from-mappings.md:17 |

## Verdict

Conformant apart from wording. No finding blocks on its own.

Examined (role examined): all 12 FR-171 Behavior bullets, all 8 FR-172 Behavior bullets, all 12 FR-173 Behavior
bullets, all 5 FR-174 Behavior bullets, and the four NFR-038 Statement SHALLs.
