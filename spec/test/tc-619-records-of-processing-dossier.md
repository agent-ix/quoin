---
id: TC-619
title: "FR-207 AC-3 unclassified scoped connection"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-619: FR-207 AC-3 unclassified scoped connection

## Description

Verify FR-207-AC-3: a connection selected by the processing activity's security-owned `scopes` edge, with no `carries` edge, produces the explicit personal-data category gap and one typed gap record.

## Test Procedure

Run `tc_1658_unclassified_connection_emits_gap_row_and_gap_record` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

The generated body names `CONN-7` in the Art 30(c) gap row, and the generated gap names `CONN-7` with kind `personal_data_categories_unstated`.
