---
id: TC-606
title: "FR-202 AC-6 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-606: FR-202 AC-6 threat-model dossier behavior

## Description

Verify FR-202-AC-6: A fully treated fixture reads `none` under `### Open findings`, `Gaps: none`, and status `complete`.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_complete_report_is_deterministic_and_has_no_gaps` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
