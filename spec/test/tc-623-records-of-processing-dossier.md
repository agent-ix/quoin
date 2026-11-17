---
id: TC-623
title: "FR-207 AC-7 missing flow facts"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-623: FR-207 AC-7 missing flow facts

## Description

Verify FR-207-AC-7: missing `quire.flow-report/v1` facts produce an explicit document gap and no substitute model source is read.

## Test Procedure

Run `tc_1658_missing_flow_report_is_an_explicit_document_gap` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The gap report contains `model_facts_unavailable` for `DOS-ROP`.
