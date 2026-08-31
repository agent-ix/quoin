---
id: SR-093
title: "Dependency review of Quoin change-assurance contracts"
type: SpecReview
analysis: dependency
scope: "FR-063..FR-065 and external Quoin/ix-flow inputs"
review_set: all
---

# SR-093: Dependency review of Quoin change-assurance contracts

## Summary

The implementation order is FR-063 canonical integrity and record storage,
FR-064 attestation intake, then FR-065 read-only verification. FR-030/032 and
ix-flow FR-013/018 are consumed contracts, not implementation work hidden in
this ticket.

## Findings

| ID      | Severity | Summary                                                                                               | Refs                                                          |
| ------- | -------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| FND-001 | low      | No dependency cycle or undeclared enablement work remains after adding the FR-018 relationship edges. | FR-063 Dependencies; FR-064 Dependencies; FR-065 Dependencies |

## Execution boundary

The Quoin lane owns schemas, raw-byte validation, content-addressed persistence,
and receipt derivation. It does not own ix-flow event production, proof
execution, evidence auditing, or downstream merge policy.
