---
id: TC-010
title: "Unavailable selected set is reported while evaluated sets continue"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: verifies
---

# TC-010: Unavailable selected set is reported while evaluated sets continue

## Description

Verify the profile-driven security-analysis behavior required by FR-171 using the named focused integration test and its synthetic profile fixtures.

## Test Procedure

1. Run `tc_171_profile_mappings::unavailable_selected_set_does_not_stop_an_evaluated_binding_set` with a profile selecting one installed and one unavailable clause set.

## Expected Results

The installed set is evaluated and retains its binding clause; the other selected set is reported `unavailable` with no report. The skill instructions require `not mapped, set not installed` and continuing other evaluated sets.
