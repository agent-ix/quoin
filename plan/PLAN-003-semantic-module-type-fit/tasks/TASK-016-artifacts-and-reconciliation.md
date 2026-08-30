---
id: TASK-016
title: "Publish canonical artifacts and reconcile contracts"
type: Task
track: "Audit publication"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-054"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-055"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-015"
    type: "depends_on"
---

# TASK-016: Publish canonical artifacts and reconcile contracts

## Status

**done** — nine canonical/generated artifacts validate by count and digest, the SpecReview passes
Quire validation, and findings link architecture, the core-data census, Quire, and gated follow-ups.

## Scope

Serialize the canonical artifact family, manifest counts/digests, conflict/missing ledgers and
repository impacts. Reconcile findings to the architecture record, core-data census, Quire corpus,
and gated follow-up classes. Generate `report.md` and a validated SpecReview from the same data.

## Exit criteria

- TC-1177..TC-1186 pass.
- Artifact validation rejects missing, stale, unreferenced, or count-disagreeing data.
- Human projections carry no independent finding or score.
