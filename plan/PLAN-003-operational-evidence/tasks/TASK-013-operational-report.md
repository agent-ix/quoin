---
id: TASK-013
title: "Render operational claims and counterevidence"
type: Task
status: todo
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: part_of
  - target: "ix://agent-ix/quoin/TASK-011"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-049"
    type: references
  - target: "ix://agent-ix/quoin/TC-1163"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1165"
    type: verifies
---

# TASK-013: Render operational claims and counterevidence

## Scope

Extend the shared report object and human/JSON renderers with distinct capability
and exercise projections under claims, evidence, counterevidence, gaps, owner, and
action.

## TDD Work

- Write TC-1163..TC-1165 before modifying projection or formatting.
- Keep unavailable/unknown/not-applicable capabilities and adverse/incomplete
  exercises out of affirmative claims.
- Derive both formats from one deterministically ordered report object.

## Exit Criteria

- Human and JSON views expose equivalent claim-centered content.
- Reordered store input is byte-stable.
- No aggregate trust, confidence, or quality score is introduced.
