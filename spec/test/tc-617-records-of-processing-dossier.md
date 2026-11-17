---
id: TC-617
title: "FR-207 AC-1 controller records-of-processing dossier"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-617: FR-207 AC-1 controller records-of-processing dossier

## Description

Verify FR-207-AC-1: a controller activity shows authored personal-data categories, recipients and explicit transfer gaps, while the sink's location and protecting control remain reported as model facts.

## Test Procedure

Run `tc_1658_controller_lists_classification_recipients_transfer_gaps_and_safeguards` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The assertions verify the personal-data classification and title, recipient and audience, a mixed `region` list with one matching and one outside home-region label, a multi-region sink whose labels all match, the general security control, and explicit transfer-region and transfer-safeguards gaps for both sinks.
