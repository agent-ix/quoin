---
id: TC-622
title: "FR-207 AC-6 invalid processing activity scopes relationships"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-207"
    type: "verifies"
---

# TC-622: FR-207 AC-6 invalid processing activity scopes relationships

## Description

Verify FR-207-AC-6: wrong-kind and dangling security-owned `scopes` targets on a `processing_activity` are rejected before a records-of-processing document is generated. ISO `covers` requirement-coverage edges do not select processing scope.

## Test Procedure

Run `tc_1658_invalid_processing_activity_scopes_fail_before_generation` in `quoin-dossier` through `/home/peter/dev/worktrees/locked-build.sh`.

## Expected Results

Wrong-kind and dangling `scopes` target diagnostics are returned as source failures; no generated dossier can claim those records as valid.
