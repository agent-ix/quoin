---
id: TC-006
title: "Unresolved profile clauses remain review findings and are not mapped"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: verifies
---

# TC-006: Unresolved profile clauses remain review findings and are not mapped

## Description

Verify the profile-driven security-analysis behavior required by FR-171 using the named focused integration test and its synthetic profile fixtures.

## Test Procedure

1. Run `tc_171_profile_mappings::fixture_a_skill_output_maps_only_binding_profile_clauses_and_preserves_context` against Fixture A.

## Expected Results

The emitted SpecReview names unresolved clauses `CP-9` and `V1.1.3` and their missing dimensions `data_class` and `chapter`; neither clause appears in any emitted control or exclusion mapping.
