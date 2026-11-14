---
id: TC-003
title: "Disposition ownership and temporary-exception expiry are enforced"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-046"
    type: "verifies"
---

# TC-003: Disposition ownership and temporary-exception expiry are enforced

## Description

Verify that disposition facts carry their required owner and approval reference,
that `decisionExpiresAt` is required only for `temporary_exception` and cannot
exceed the attestation expiry, and that an expired temporary exception leaves
its binding clause open with the typed expiry state.

## Test Procedure

1. Run `disposition_wire_fields_require_an_owner_and_decision_specific_expiry`
   in `quoin-assurance/tests/discharge.rs` against the synthetic clause binding.
2. Run
   `temporary_exception_is_current_before_its_expiry_and_open_at_and_after_it`
   and `temporary_exception_shared_attestation_expiry_is_classified_as_expired`
   in the same test target.

## Expected Results

1. `delegated` and `accepted_risk` retain their owner and approval reference,
   omit `decisionExpiresAt`, and refuse a supplied expiry. A missing owner or
   approval reference is refused. `temporary_exception` requires its own
   expiry, and an expiry later than the attestation is refused.
2. A temporary exception is a disposition immediately before its decision
   expiry. At and after the expiry it is open with reason
   `temporary exception expired`; the same typed RMM expiry classification is
   emitted when the decision and attestation expiries are equal.
