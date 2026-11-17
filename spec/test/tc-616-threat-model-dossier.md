---
id: TC-616
title: "FR-202 AC-16 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-616: FR-202 AC-16 threat-model dossier behavior

## Description

Verify FR-202-AC-16: A `threat_model` or `threat` file outside the selected corpus is not read: it yields no model section and no threat cell entry.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_uses_only_the_selected_corpus` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
