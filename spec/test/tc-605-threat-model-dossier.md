---
id: TC-605
title: "FR-202 AC-5 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-605: FR-202 AC-5 threat-model dossier behavior

## Description

Verify FR-202-AC-5: A selection with no `threat_model` writes `DOS-TM.md` with no model section, one `no_threat_model` gap, and status `incomplete`; the three checks produce no finding.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_report_without_model_has_one_document_gap` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
