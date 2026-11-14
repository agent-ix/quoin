---
id: TC-010
title: "Invalid profile evaluation stops analysis before mapping"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: verifies
---

# TC-010: Invalid profile evaluation stops analysis before mapping

## Description

Verify the profile-driven security-analysis behavior required by FR-171 using the named focused integration test and its synthetic profile fixtures.

## Test Procedure

1. Run `tc_171_profile_mappings::skill_specifies_unresolved_unavailable_immutability_and_fail_closed_behavior` and inspect the skill failure handling.

## Expected Results

The skill reports nonzero evaluation, missing output, or schema-invalid output and stops without mapping or authoring controls or exclusions.
