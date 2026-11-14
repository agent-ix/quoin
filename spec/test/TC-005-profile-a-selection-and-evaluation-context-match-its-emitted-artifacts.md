---
id: TC-005
title: "Profile A selection and evaluation context match its emitted artifacts"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: verifies
---

# TC-005: Profile A selection and evaluation context match its emitted artifacts

## Description

Verify the profile-driven security-analysis behavior required by FR-171 using the named focused integration test and its synthetic profile fixtures.

## Test Procedure

1. Run `tc_171_profile_mappings::fixture_a_skill_output_maps_only_binding_profile_clauses_and_preserves_context` against Fixture A.

## Expected Results

The evaluation contains exactly `nist/sp800-53@5.2.0` and `owasp/asvs@5.0.0`, with `baseline=low` and `level=1`; each emitted mapping uses one of those keys.
