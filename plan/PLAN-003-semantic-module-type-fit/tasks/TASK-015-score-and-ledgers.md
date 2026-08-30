---
id: TASK-015
title: "Score semantic fit and build finding ledgers"
type: Task
track: "Audit engine"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-053"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-014"
    type: "depends_on"
---

# TASK-015: Score semantic fit and build finding ledgers

## Status

**done** — all 90 declaration records carry all 11 axes, evidence, confidence, and one disposition;
the retained run reports 16 fits and 74 incomplete declarations, including 60 placeholder schemas.

## Scope

Implement pure per-axis assessment and disposition derivation, placeholder/blob/occurrence-signal
detection, module-qualified duplicate comparison, and conflict/missing-concept ledgers with evidence and confidence.

## Exit criteria

- TC-1169..TC-1176 pass.
- Every declaration has every required axis and exactly one disposition.
- Heuristic evidence never claims greater confidence than its input supports.
