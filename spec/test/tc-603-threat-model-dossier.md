---
id: TC-603
title: "FR-202 AC-3 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-603: FR-202 AC-3 threat-model dossier behavior

## Description

Verify FR-202-AC-3: A considered threat with no control and no accepted risk yields `GAP: threat-without-treatment` and one `threat_without_treatment` gap; with a risk `decision: undecided` it still does, and with `decision: accepted` it does not.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_renders_only_engine_findings_with_severity` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
