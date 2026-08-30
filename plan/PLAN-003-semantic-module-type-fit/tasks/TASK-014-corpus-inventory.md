---
id: TASK-014
title: "Inventory the complete module corpus"
type: Task
track: "Audit engine"
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: "part_of"
  - target: "ix://agent-ix/quoin/FR-052"
    type: "references"
  - target: "ix://agent-ix/quoin/TASK-013"
    type: "depends_on"
---

# TASK-014: Inventory the complete module corpus

## Status

**done** — the retained census closes 10 modules, 90 declarations, 450 contract-surface states,
and 299 Markdown paths (236 parsed and 63 explicitly untyped).

## Scope

Enumerate every manifest entry, artifact/object declaration, contract surface, and Markdown path.
Invoke Quire read-only where available, retain all five parse states, extract stable identities and
occurrence signals, attach representative instances, and reconcile every denominator.

## Exit criteria

- TC-1162..TC-1168 pass.
- No duplicate, absent surface, missing instance, excluded file, or parser failure is silently dropped.
- The complete inventory is deterministic and path-portable.
