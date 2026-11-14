---
id: TC-009
title: "Missing profile stops analysis before mapping"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: verifies
---

# TC-009: Missing profile stops analysis before mapping

## Description

Verify the profile-driven security-analysis behavior required by FR-171 using the named focused integration test and its synthetic profile fixtures.

## Test Procedure

1. Run `tc_171_profile_mappings::no_profile_instruction_stops_before_mapping_and_does_not_claim_defaults_are_installed` with `.ix/source-profile.yaml` removed.

## Expected Results

Evaluation reports that no source profile is selected, emits no evaluation document or mapping artifacts, and the skill instructs the user to create the profile file without defaulting to all sources.
