---
id: TC-621
title: "FR-207 AC-5 processor record layout"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-621: FR-207 AC-5 processor record layout

## Description

Verify FR-207-AC-5: the processor activity shows its own contact and controllers, each scoped sink's kind under Art 30(2)(b), and any sink purpose separately.

## Test Procedure

Run `tc_1658_processor_uses_article_30_2_rows_without_controller_only_entries` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The Art 30(2) output includes the processor and controller contacts, category of processing and separate purpose context; controller-only rows are absent.
