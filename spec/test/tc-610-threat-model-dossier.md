---
id: TC-610
title: "FR-202 AC-10 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-610: FR-202 AC-10 threat-model dossier behavior

## Description

Verify FR-202-AC-10: A connection covered by two models and lacking a threat gets a `GAP:` row in both sections and one `crossing_without_threat` gap; a considered threat with no treatment gets its `GAP:` row in the section of the model that considers it.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_treatment_findings_require_considers_and_scope_edges` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
