---
id: TC-613
title: "FR-202 AC-13 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-613: FR-202 AC-13 threat-model dossier behavior

## Description

Verify FR-202-AC-13: A threat considered by two models and lacking treatment gets its `GAP:` row in each model's section and one `threat_without_treatment` gap.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_treatment_findings_require_considers_and_scope_edges` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
