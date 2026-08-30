---
id: PLAN-003
title: "Operational evidence implementation"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/quoin/US-014"
    type: references
  - target: "ix://agent-ix/quoin/FR-048"
    type: references
  - target: "ix://agent-ix/quoin/FR-049"
    type: references
  - target: "ix://agent-ix/quoin/FR-051"
    type: references
---

# PLAN-003: Operational evidence implementation

## Outcome

Deliver Quoin #271: engine-independent standing-capability and exercise records,
governed intake and verified clock discharge, claim-centered reporting, and a
first-party producer for retained real GitHub Actions release evidence.

## Scope Boundaries

- Quoin owns schema/semantic validation, evidence-store persistence, obligation
  matching, deterministic clock interpretation, reporting, and retained-artifact
  adaptation.
- GitHub Actions owns workflow execution, API observations, and release effects.
- The adapter performs no network call, process execution, workflow dispatch,
  release publication, drill, or operational-control alteration.
- Quoin #286 and `filament-ide-rs` are outside this plan.

## Dependency Graph

```text
TASK-011 record family
  ├── TASK-012 intake and clock discharge
  └── TASK-013 report projection
TASK-011 + TASK-012
  └── TASK-014 GitHub release producer
TASK-012 + TASK-013 + TASK-014
  └── TASK-015 integration and compatibility gate
```

TASK-012 and TASK-013 may proceed in parallel after TASK-011. The producer reuses
the engine-independent intake boundary and does not couple core records to GitHub.

## Test Strategy

- Unit/property tests lead schema, cross-record, temporal, pin, discharge, and
  adverse-state behavior.
- Store tests verify definition gating, raw bytes, atomicity, idempotency, and
  collisions on temporary same-filesystem directories.
- Producer tests structurally parse workflow YAML and retained GitHub API JSON and
  derive all decision-bearing fields from those artifacts.
- The final integration captures a real release run outside Quoin, then proves the
  adapter is offline, process-free, and all-or-nothing at intake.

## Task File Mapping

| Task     | Track | Scope                                     | Owns                                      |
| -------- | ----- | ----------------------------------------- | ----------------------------------------- |
| TASK-011 | A     | Record shapes and semantic invariants     | FR-048; TC-1147..TC-1155                  |
| TASK-012 | B     | Governed intake and clock discharge       | FR-049 intake/discharge; TC-1156..TC-1162 |
| TASK-013 | C     | Claim-centered operational report         | FR-049 reporting; TC-1163..TC-1165        |
| TASK-014 | D     | First-party GitHub release producer       | FR-051; TC-1173..TC-1176                  |
| TASK-015 | Gate  | No-control, compatibility, real-run gates | TC-1166; TC-1167; TC-1177                 |

## Completion Gates

- Every mapped test carries the owning requirement tracking tag.
- `quire validate --scope . "spec/**/*.md" "plan/**/*.md"` passes.
- Typecheck, lint, unit/property/integration tests, and coverage gates pass.
- Gap analysis reports no incomplete task, unbacked matrix row, or unowned code
  before #271 is declared done.
