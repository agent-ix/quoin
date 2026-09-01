---
id: TASK-008
title: "Render intervention claims and evidence"
type: Task
status: done
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-006"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-057"
    type: references
  - target: "ix://agent-ix/quoin/TC-1210"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1213"
    type: verifies
---

# TASK-008: Render intervention claims and evidence

## Scope

Extend the shared report object and both renderers with intervention conclusions,
effects, counterevidence, gaps, owners, actions, and raw references.

## TDD Work

- Write TC-1210..TC-1213 before changing report projection or formatting.
- Build one deterministically ordered projection consumed by human and JSON output.
- Preserve unknown/uncontrolled qualifiers and negative results without deriving a
  trust score or causal conclusion.

## Exit Criteria

- Both renderers expose equivalent claim-centered content.
- Reordered store inputs render byte-identically.
- No aggregate trust/confidence/quality score or inferred causality appears.
