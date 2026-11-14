---
id: TC-006
title: "Security analysis leaves profile selection and context unchanged"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: verifies
---

# TC-006: Security analysis leaves profile selection and context unchanged

## Description

Verify the profile-driven security-analysis behavior required by FR-171 using the named focused integration test and its synthetic profile fixtures.

## Test Procedure

1. Run `tc_171_profile_mappings::fixture_a_skill_output_maps_only_binding_profile_clauses_and_preserves_context` and compare `.ix/source-profile.yaml` bytes before and after evaluation.

## Expected Results

The source profile and its context are unchanged by evaluation; the skill does not author profile selection or product facts.
