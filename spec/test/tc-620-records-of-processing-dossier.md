---
id: TC-620
title: "FR-207 AC-4 transfer gaps remain explicit"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-620: FR-207 AC-4 transfer gaps remain explicit

## Description

Verify FR-207-AC-4: a covered sink retains both transfer gaps even when its location and a protecting control with safeguards are present.

## Test Procedure

Run `tc_1658_controller_lists_classification_recipients_transfer_gaps_and_safeguards` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The sink's location is rendered as a supplemental model fact, transfer safeguards remain a gap, and control safeguards are shown only as general security measures.
