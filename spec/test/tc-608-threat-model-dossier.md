---
id: TC-608
title: "FR-202 AC-8 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-608: FR-202 AC-8 threat-model dossier behavior

## Description

Verify FR-202-AC-8: The document holds no threat, STRIDE class, `decision` or `decided_by` value that does not appear in a record, and no score, rank or adequacy statement; a risk with no `decided_by` shows `not stated`.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_renders_authored_facts_without_inference` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
