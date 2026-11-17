---
id: TC-622
title: "FR-207 AC-6 invalid covers relationships"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-622: FR-207 AC-6 invalid covers relationships

## Description

Verify FR-207-AC-6: wrong-kind and dangling `covers` targets are rejected before a records-of-processing document is generated.

## Test Procedure

Run `tc_1658_invalid_covers_edges_fail_before_generation` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

Wrong-kind and dangling target diagnostics are returned as source failures; no generated dossier can claim those records as valid.
