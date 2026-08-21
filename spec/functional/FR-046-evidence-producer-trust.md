---
id: FR-046
title: "Use-specific evidence-producer trust decisions and invalidation"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
---

# FR-046: Use-specific evidence-producer trust decisions and invalidation

## Description

Quoin records why one bounded decision may rely on one evidence-producer use, then makes
changes to the accepted context visible without awarding the producer a global badge.

A decision identifies its intended use, permitted downstream decisions, validation
evidence, accepted limitations, and accountable owner. It preserves accepted and observed
producer, configuration, adapter, validation-corpus, input-contract, and environment
identity. The observed context is explicit: its absence cannot be interpreted as agreement.

Quoin SHALL compute the resulting assessment from the accountable decision and the selected
revalidation triggers. An accepted decision with matching context is accepted (or accepted
with limitations); a selected mismatch is invalidated; no observation is unobserved; an
explicit refusal is not accepted. The same producer can therefore be acceptable for an
advisory review use and unacceptable for automatic release approval.

`quoin evidence trust` transcribes completed JSON into the canonical evidence store. It does
not run or validate the producer. `quoin assurance` renders the assessment as context; the
assessment does not turn any claim into a supported claim.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-046-AC-1 | Quoin identifies each trust decision, intended producer use, permitted downstream decisions, decision owner, and acceptance time. | Test (TC-297) |
| FR-046-AC-2 | Quoin preserves producer, version, configuration, adapter, validation-corpus, input-contract, environment, validation-evidence, and limitation identity without naming a supported analyzer in engine policy. | Test (TC-297) |
| FR-046-AC-3 | When an observed context differs from an accepted context on a selected trigger, Quoin reports the decision as invalidated and names every changed trigger. | Test (TC-298) |
| FR-046-AC-4 | When an accepted decision has no observed context, Quoin reports it as unobserved rather than accepted or rejected. | Test (TC-299) |
| FR-046-AC-5 | Quoin permits two uses of one producer to carry different reliance decisions. | Test (TC-300) |
| FR-046-AC-6 | Quoin stores valid decisions canonically and reports unreadable or invalid stored decisions without hiding the remaining decisions. | Test (TC-301) |
| FR-046-AC-7 | Quoin renders trust assessments as assurance context without altering any claim status. | Test (TC-302) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-046-CON-1 | Quoin SHALL NOT infer trust from producer name, vendor, version, or absence of a decision. | Design | Inspection |
| FR-046-CON-2 | Quoin SHALL NOT run, certify, approve, or globally qualify a producer. | Design | Inspection |
| FR-046-CON-3 | Quoin SHALL NOT silently convert invalidated or unobserved reliance to accepted reliance. | Design | Test (TC-298, TC-299) |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) for canonical machine records and [FR-040](./FR-040-assurance-case.md) for the read-only assurance view.
- **Downstream**: profile-selected independent verification and release-policy consumers may use the effective state; absence remains neutral.
