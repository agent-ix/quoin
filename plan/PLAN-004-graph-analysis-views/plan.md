---
id: PLAN-004
title: "Graph-analysis views"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: references
  - target: "ix://agent-ix/quoin/US-018"
    type: references
  - target: "ix://agent-ix/quoin/FR-062"
    type: references
---

# PLAN-004: Graph-analysis views

## Requirements Summary

### Stakeholder Requirements

- [ ] **StR-004**: Govern spec workflows with inspectable evidence.

### User Stories

- [ ] **US-018**: Inspect evidence-graph concentration and change exposure.

### Functional Requirements

- [ ] **FR-062**: Derive deterministic, read-only fan-out, change-impact, and churn reports from an
      accepted Quire assurance export, retained evidence, and unchanged auditor verdicts.

## Dependency Graph

- `quire-rs FR-067 + FR-068 -> TASK-020`
  Reason: Quoin must consume the hand-authored assurance-v1 schema and source-grounded records;
  inventing or re-parsing that contract is prohibited.
- `FR-030 + FR-032 + TASK-020 -> TASK-021`
  Reason: fan-out and churn join live exported obligations with retained bindings and affirmations.
- `TASK-021 -> TASK-022`
  Reason: change impact reuses the normalized artifact, obligation, suite, and verdict indexes.
- `TASK-020 + TASK-021 + TASK-022 -> TASK-023`
  Reason: the command and renderers must expose the settled report model rather than define it.
- `TASK-023 -> TASK-024`
  Reason: static boundary and legacy byte-compatibility gates seal the complete production path.

### Shared dependencies

The validated assurance-export reader and normalized immutable indexes are shared by all three
views. They are single implementations owned by TASK-020/TASK-021; later tasks consume them rather
than rebuilding a graph or reading specification frontmatter.

### Cross-cutting constraints

FR-062-CON-1..CON-4 apply to every task: no producer, suite, Quire, Git, network, write, scoring,
verdict mutation, frontmatter read, or independently constructed artifact graph may enter the
analysis path.

### The seams

The work attaches at `src/quire/` for schema-validated import, `src/evidence/` and `src/auditor/` for
existing retained operands/verdicts, `src/graph-analysis/` for pure projections, and
`src/commands/graph.ts` plus `vite.config.ts` for the oclif surface.

## Test Plan

### Property tests

- [ ] **TC-1249**: fan-out set/count is duplicate- and input-order-invariant.
- [ ] **TC-1251**: relation selection, reverse closure, cycles, shared dependents, and shortest-path
      tie-breaking are deterministic.
- [ ] **TC-1254**: one copied affirmation is one event while all affected suites remain visible.
- [ ] **TC-1255**: zero-event rows, absent-obligation gaps, and churn ordering hold under permutations.
- [ ] **TC-1256**: source and module premises survive repeated analysis byte-for-byte.
- [ ] **TC-1258**: equivalent input permutations produce byte-identical JSON and matching human output.

### Unit tests

- [ ] **TC-1250**: unresolved bindings stay named under their suite and in gaps but do not count live.
- [ ] **TC-1252**: every reached requirement joins obligations/suites; unknown seeds return no partial
      closure.
- [ ] **TC-1253**: change exposure and every unchanged FR-032 verdict remain separate.
- [ ] **TC-1257**: invalid, absent, unreadable, incomplete, and valid-empty inputs retain distinct states.

### Static and integration tests

- [ ] **TC-1259**: dependency boundaries reject execution, write, frontmatter, and second-graph paths.
- [ ] **TC-1260**: legacy evidence/assurance/measurement outputs remain byte-compatible and graph
      output contains no score or threshold classification.

## Remaining Work

### Track A: Critical path (serial)

- **A1 = TASK-020** assurance-export consumer — Hard; exit: only schema-valid, accepted-premise
  exports expose typed records.
- **A2 = TASK-021** fan-out and churn — Medium; exit: live/unresolved bindings and deduplicated
  affirmation events produce complete deterministic rows.
- **A3 = TASK-022** change impact — Hard; exit: typed reverse closure returns deterministic paths,
  joined evidence, and unchanged auditor verdicts.
- **A4 = TASK-023** command and rendering — Medium; exit: all three subcommands emit the same report
  objects through JSON and human renderers.
- **Gate = TASK-024** boundary and regression seal — pass: TC-1259/TC-1260 plus the full repository
  verification stack are green.

## Parallel Execution Summary

```text
TASK-020 -> TASK-021 -> TASK-022 -> TASK-023 -> TASK-024
 contract     indexes     closure      CLI/render    seal
```

The external quire-rs FR-067/FR-068 implementation can proceed independently before TASK-020;
inside Quoin, the shared report model and overlapping files make the remaining work intentionally
serial.

## Task File Mapping

| Task     | Track | Owns (references) | Verified by (verifies)             | Status      |
| -------- | ----- | ----------------- | ---------------------------------- | ----------- |
| TASK-020 | A     | FR-062            | TC-1256, TC-1257                   | not_started |
| TASK-021 | A     | FR-062            | TC-1249, TC-1250, TC-1254, TC-1255 | not_started |
| TASK-022 | A     | FR-062            | TC-1251, TC-1252, TC-1253          | not_started |
| TASK-023 | A     | FR-062            | TC-1258                            | not_started |
| TASK-024 | Gate  | FR-062            | TC-1259, TC-1260                   | not_started |

## Coordination Rules

- Consume the exact assurance-v1 schema and types delivered by quire-rs #386; do not infer a shape
  from its specification or revive PR #190's frontmatter reader/Quire subprocess.
- Keep FR-032 verdicts opaque and unchanged; graph reachability is a separate fact.
- Preserve the report model as the single source for JSON and human output.
- Land #152 before #281 rebases its portfolio integration onto FR-062 reports.
