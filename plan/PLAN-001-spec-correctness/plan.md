---
id: PLAN-001
title: "Phase C — the spec-correctness skill"
type: Plan
relationships:
  - target: "ix://agent-ix/quoin/FR-028"
    type: "references"
  - target: "ix://agent-ix/quoin/US-011"
    type: "references"
---

# PLAN-001: Phase C — the spec-correctness skill

> **Authored retroactively.** This bundle did not guide the work; it records what
> was actually built, so the plan-completion gate of `gap-analysis` can run over
> FR-028 at all. SR-003 FND-004 raised its absence. Nothing here was written
> before the code, and the task statuses are observations, not forecasts.

## Scope

Build and land the `spec-correctness` skill: the consumer of the per-criterion
property classification `quire properties --json` emits (quire-rs FR-052), which
turns acceptance criteria into runnable property tests.

Out of scope: any change to the classifier itself, and any widening of its
deterministic recall — that ceiling is measured and accepted (quire-rs#45).

## Task File Mapping

| Task     | Scope                                                         | Owns                     |
| -------- | ------------------------------------------------------------- | ------------------------ |
| TASK-001 | Author FR-028 + US-011 and the eval scenarios EV-050..EV-053  | FR-028, US-011           |
| TASK-002 | Build the skill: SKILL.md, step references, harness templates | skills/spec-correctness/ |
| TASK-003 | Dogfood on quoin and quire-rs; emit and accept the tests      | tests/props/             |
| TASK-004 | Reconcile the suite: tracking tags for every tested criterion | tests/\*.test.ts         |
| TASK-005 | Land it: merge, release, verify consumed                      | @agent-ix/quoin@0.12.0   |
