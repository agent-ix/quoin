---
id: TC-614
title: "FR-202 AC-14 threat-model dossier behavior"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-202"
    type: "verifies"
---

# TC-614: FR-202 AC-14 threat-model dossier behavior

## Description

Verify FR-202-AC-14: A fixture with two connections, two sinks, three findings of different checks and two part ids in non-sorted source order yields rows, entries, findings and `## Gaps` in the Ordering above; rows, entries, findings and gaps follow dossier order; a selection without a model yields only the `no_threat_model` gap.

## Test Procedure

Run `tc_1546_threat_model::tc_1546_findings_and_gaps_follow_check_order` in `quoin-dossier` using `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The integration test asserts the criterion against the threat-model dossier renderer and the selected typed records and flow findings.
