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

Verify profile selection and context are unchanged across the live security-analysis skill runs that generate the checked-in Fixture A and B artifacts.

## Test Procedure

1. Run `tests/scripts/run_security_profile_skill_fixtures.sh` with the installed synthetic profile and authoring modules configured. The script evaluates each profile, snapshots `.ix/source-profile.yaml` and the evaluated context immediately before the live `codex exec`, then snapshots both immediately after it; it stores the snapshots alongside the emitted artifacts.
2. Run `tc_171_profile_mappings::live_fixture_skill_runs_preserve_selection_and_evaluated_context`.

## Expected Results

For each live fixture run, before/after selection bytes and evaluated context are identical. The snapshots describe the same Codex runs whose controls, exclusions and SpecReviews are checked by TC-003 through TC-005 and TC-007.
