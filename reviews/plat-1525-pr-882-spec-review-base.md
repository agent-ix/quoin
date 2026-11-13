---
id: SR-20261113181012
title: "Base requirements review — PLAT-1525 PR #882 dispositions"
type: SpecReview
analysis: base
scope: "spec/functional/FR-046-clause-discharge.md, FR-173-facts-from-mappings.md, FR-174-per-clause-compliance-from-mappings.md"
review_set: subset
---

## Summary

The review covered the updated discharge, mapping and RMM requirements. The authored temporary-exception expiry is not explicitly represented or enforced through the stored-fact path, and a successful report line describes metadata that upstream validation and this command reject.

## Verdict

**FAIL** — a temporary exception can outlive its authored limit, and one specified success-report state conflicts with mandatory input validation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Temporary-exception expiry is not specified through persistence and evaluation | FR-173-AC-12, FR-046-AC-7, FR-174-AC-2 |
| FND-002 | medium | needs-disposition is described as a successful report for invalid metadata | FR-173 |

### FND-001 — Temporary-exception expiry is not specified through persistence and evaluation

FR-016 makes expires_at an authored property for temporary_exception, and FR-046 says that decision is bounded by that expiry and the attestation interval. FR-173-AC-10 limits the authored instant to the attestation expiry. However, FR-173-AC-12 requires only persistence and rendering of the three decision labels; it does not require storing the authored expiry in a named fact field or making the clause open once --as-of reaches it. The existing FR-046 disposition fact contract has no decision-expiry field, while FR-133 persists that fact unchanged. An implementation can therefore keep a temporary exception active until the later attestation expiry. Specify the fact field and exact expiry boundary, then add an end-to-end criterion that advances --as-of past the authored decision expiry and verifies the clause is open.

### FND-002 — needs-disposition is described as a successful report for invalid metadata

The report section says its lines are emitted on success, yet needs-disposition is defined for an exclusion missing authored disposition metadata. Upstream FR-016 requires the decision, decision owner and approval reference, while FR-173 requires missing or malformed metadata to refuse the command; an invalid --spec also refuses. Thus a valid successful corpus cannot reach the described state. Remove the success-report line or define it as a refusal diagnostic with a corresponding criterion.

## Scope

- Reviewed the three changed functional requirements and their linked FR-133 and spec-objects-security FR-016 contracts.
- Full-repository validation is recorded in the integrity review; the changed FRs validate individually.

## Dispositions

Round 1, verified at 831d366dbe90bd3ef0ee905d189ead44752295e.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-046 defines top-level decisionExpiresAt; FR-133 persists and evaluates it; FR-174-AC-6 tests immediately before, at, and after expiry. |
| FND-002 | fixed | The needs-disposition success line is removed; FR-173-AC-13 verifies missing or invalid required metadata produces non-zero status and empty stdout. |
