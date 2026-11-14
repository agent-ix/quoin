---
id: TC-005
title: "Profile A emitted mappings are binding clauses from selected sets"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: verifies
---

# TC-005: Profile A emitted mappings are binding clauses from selected sets

## Description

Verify the profile-driven security-analysis behavior required by FR-171 using the named focused integration test and its synthetic profile fixtures.

## Test Procedure

1. Run `tc_171_profile_mappings::fixture_a_skill_output_maps_only_binding_profile_clauses_and_preserves_context`. It evaluates the selected profile and reads the control and scope-exclusion artifacts emitted by the security-analysis skill into Fixture A and Fixture B.

## Expected Results

Every emitted mapping resolves to a set in its evaluation and a clause reported `binding`. No `not_binding` or `unresolved` clause is mapped, and Fixture B has no mapping for the NIST set absent from its evaluation.
