---
id: TC-618
title: "FR-207 AC-2 authored fields and processor layout"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-618: FR-207 AC-2 authored fields and processor layout

## Description

Verify FR-207-AC-2: an unusable controller activity fails before generation, and a processor activity without retention validates for generation and omits entry (f).

## Test Procedure

Run `tc_1658_unusable_activity_fields_fail_before_generation` and `tc_1658_processor_uses_article_30_2_rows_without_controller_only_entries` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The controller validation failure is returned as a source failure. The processor output contains the Art 30(2) rows and no retention row.
