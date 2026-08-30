---
id: PLAN-002
title: "Intervention-experiment evidence implementation"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/quoin/US-013"
    type: references
  - target: "ix://agent-ix/quoin/FR-046"
    type: references
  - target: "ix://agent-ix/quoin/FR-047"
    type: references
  - target: "ix://agent-ix/quoin/FR-050"
    type: references
---

# PLAN-002: Intervention-experiment evidence implementation

## Outcome

Deliver Quoin #270: a versioned intervention-experiment record, governed and
raw-byte-verified intake, claim-centered reporting, and a first-party adapter for
two retained real `cli-agent-evals` runs.

## Scope Boundaries

- Quoin owns schema and semantic validation, evidence-store persistence, report
  projection, and retained-report adaptation.
- `cli-agent-evals` owns evaluation execution and report production.
- Quoin SHALL NOT run an experiment, invoke an agent, or infer causality that the
  retained design and observations cannot establish.
- Existing evidence and measurement collections remain readable without migration.

## Dependency Graph

```text
TASK-006 record contract
  ├── TASK-007 governed intake
  └── TASK-008 report projection
TASK-006 + TASK-007
  └── TASK-009 agent-eval producer
TASK-007 + TASK-008 + TASK-009
  └── TASK-010 integration and compatibility gate
```

TASK-007 and TASK-008 may proceed in parallel after TASK-006. TASK-009 consumes
the contract and intake boundary but does not depend on report rendering.

## Test Strategy

- Unit and property tests lead schema, semantic, causal-safety, and deterministic
  projection work.
- Store tests use temporary same-filesystem directories to verify atomic rename,
  idempotency, collision refusal, and raw-byte integrity.
- Static tests prove intake, reporting, and the adapter have no experiment/process
  execution path.
- The final E2E runs `cli-agent-evals` outside Quoin, retains both reports, and then
  invokes only Quoin's adapter/intake/report surfaces.

## Task File Mapping

| Task     | Track | Scope                                            | Owns                                     |
| -------- | ----- | ------------------------------------------------ | ---------------------------------------- |
| TASK-006 | A     | Record types, JSON Schema, semantic validator    | FR-046; TC-1125..TC-1133                 |
| TASK-007 | B     | Governed intake, raw evidence, atomic store      | FR-047 intake; TC-1134..TC-1139; TC-1146 |
| TASK-008 | C     | Claim-centered human/JSON projection             | FR-047 reporting; TC-1140..TC-1143       |
| TASK-009 | D     | First-party agent-eval intervention producer     | FR-050; TC-1168..TC-1171                 |
| TASK-010 | Gate  | Real-run E2E, no-process and compatibility gates | TC-1144; TC-1145; TC-1172                |

## Completion Gates

- Every mapped test carries the owning requirement tracking tag.
- `quire validate --scope . "spec/**/*.md" "plan/**/*.md"` passes.
- Typecheck, lint, unit/property/integration tests, and coverage gates pass.
- A gap analysis finds every plan task complete and every criterion backed by real
  evidence before #270 is declared done.
