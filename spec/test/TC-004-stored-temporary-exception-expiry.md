---
id: TC-004
title: "A stored temporary exception expires at its decision boundary"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-133"
    type: "verifies"
  - target: "ix://agent-ix/quoin/FR-174"
    type: "verifies"
---

# TC-004: A stored temporary exception expires at its decision boundary

## Description

Verify that `record-discharge` persists a temporary exception's
`decisionExpiresAt`, and that `discharge --repo --rmm` reads the stored fact to
render it as current before the decision expiry and expired at and after it,
while its attestation remains current.

## Test Procedure

1. Run `tc_2606_a_stored_temporary_exception_expires_at_its_decision_boundary`
   in `quoin-cli/tests/tc_2606_discharge_store_command.rs` with the synthetic
   `clause-binding-v1` document and one temporary-exception fact.
2. Record the fact with `quoin evidence record-discharge`, then run
   `quoin discharge --repo --rmm` immediately before the decision expiry, at
   that instant, and immediately after it.

## Expected Results

1. The stored fact retains `decisionExpiresAt` unchanged.
2. Immediately before expiry, the RMM row is `Tailored`. At and after expiry,
   the stored clause is open and the RMM row is
   `Not compliant - temporary exception expired`, even though the attestation
   remains current.
