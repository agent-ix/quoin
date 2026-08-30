---
id: PLAN-002
title: "Semantic-module architecture and ownership record"
type: Plan
relationships:
  - target: "ix://agent-ix/quoin/US-013"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-047"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-048"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-050"
    type: "references"
  - target: "ix://agent-ix/quoin/NFR-013"
    type: "references"
  - target: "ix://agent-ix/quoin/NFR-014"
    type: "references"
---

# PLAN-002: Semantic-module architecture and ownership record

## Objective

Implement issue #289 as a tested, indexed, architecture-only record that applies the
`filament-core-data` semantic-data model to Quoin, Quire, dynamic modules, finite generated
packages, and consuming systems. Stop at the named Quoin/Quire maintainer-review gate before merge;
record its disposition without broadening it into downstream implementation authority.

## Requirements in scope

- [x] **US-013:** A maintainer can reason about definitions, occurrences, projections, and owners.
- [x] **FR-046:** Record the meta, definition, execution/observation, and presentation planes.
- [x] **FR-047:** Allocate Quire, Quoin, compiler, module, and consumer ownership.
- [x] **FR-048:** Declare authority, edit direction, and provenance by concern.
- [x] **FR-049:** Preserve both dynamic module loading and finite native generated packages.
- [x] **FR-050:** Reconcile Quire ADR-0002, ADR-0003, ADR-0004, ADR-0011, and current spec boundaries.
- [x] **NFR-013:** Keep every normative claim standalone-readable and traceable.
- [x] **NFR-014:** Change no current behavior and require maintainer review before merge; TC-1155
      was satisfied by named active maintainer `kreneskyp`'s admin merge of PR #311.

## Scope boundaries

### In scope

- Requirements, Test Matrix rows, composite specification reviews, and this plan bundle.
- `docs/semantic-module-architecture/` with an index, normative plane/authority/ownership records,
  a dynamic-versus-generated compatibility record, an external-decision ledger, and local ADRs.
- A Vitest architecture-contract suite with exact TC/AC trace tags and a changed-path guard.
- Full existing repository validation plus code review and gap analysis.

### Out of scope

- Production TypeScript behavior, Quire parser behavior, module manifests, schemas, skeletons,
  compiler or emitter code, generated language packages, registry publication, PostgreSQL models,
  wire protocols, migrations, and consumer changes.
- Promoting TypeSpec, editing `filament-core-data` ADR-0004, editing Quire ADR status, or retiring
  current Avro contracts.
- Merging without named Quoin/Quire maintainer review.

## Inputs and accepted premises

| Input                                             | Status used by this plan                                                                                          |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `filament-core-data#8` semantic-data architecture | Merged normative foundation.                                                                                      |
| `filament-core-data#4` feasibility evidence       | Accepted evidence recommends modular JSON Schema 2020-12 for the next stage; TypeSpec promotion remains held.     |
| Quire current specification                       | Canonical Markdown at the document boundary; render removed; parse/validate/extract/address/byte-splice retained. |
| Quire ADR-0002                                    | Draft and partly historical where it allocates rendering to Quire; byte-splice pipeline content retained.         |
| Quire ADR-0003 and ADR-0004                       | Proposed structural/direct-Markdown decisions, preserved without promotion.                                       |
| Quire ADR-0011                                    | Accepted governing validation-level and capability-role boundary.                                                 |

## Architecture of the deliverable

```text
requirements + external decisions
              │
              v
docs/semantic-module-architecture/index.md
├── planes-and-authority.md
├── ownership-and-boundaries.md
├── dynamic-and-generated.md
├── decision-ledger.md
└── adr/
    ├── 0001-authority-by-concern.md
    └── 0002-preserve-quire-quoin-boundaries.md
              │
              v
tests/semantic-module-architecture.test.ts
              │
              v
Quire validation + existing repository gates + review artifacts
```

The architecture index is the reading entry point. Normative topical records state the model.
Local ADRs state why the authority and ownership choices were made. The ledger records the exact
status and identity of each external decision without attempting to change that external decision.

## TDD and evidence strategy

1. Add a failing architecture-contract suite for TC-1125..TC-1154 before the architecture files.
2. Make the tests pass by writing the smallest complete record and local ADRs.
3. Run targeted Quire validation, full Quire validation, formatting, type checking, build, tests,
   coverage, and the repository's normal careful gate where practical.
4. Change matrix statuses from planned to covered only after exact `// Trace:` tags bind to tests.
5. Run `/code-review` and `/gap-analysis`; fix all blocking findings and rerun their evidence.
6. Open the PR and stop at TC-1155 for named maintainer review before merge.

## Test Matrix allocation

| Cases            | Evidence                                                               | Owner task         |
| ---------------- | ---------------------------------------------------------------------- | ------------------ |
| TC-1125..TC-1128 | Four planes, distinctions, non-goal, and indexed status/gates          | TASK-006, TASK-007 |
| TC-1129..TC-1133 | Positive and negative ownership plus ADR-0011 roles                    | TASK-006, TASK-008 |
| TC-1134..TC-1139 | Authority matrix, schema-source status, representations, conflict stop | TASK-006, TASK-007 |
| TC-1140..TC-1144 | Dynamic/open and finite/static coexistence                             | TASK-006, TASK-009 |
| TC-1145..TC-1150 | Quire decision reconciliation and external identity                    | TASK-006, TASK-008 |
| TC-1151..TC-1154 | Ledger completeness, links, status integrity, changed-path guard       | TASK-006, TASK-010 |
| TC-1155          | Named maintainer review before merge                                   | TASK-011           |

## Execution order and dependencies

```text
TASK-006 red contract tests
    ├──> TASK-007 planes + authority
    ├──> TASK-008 ownership + decision reconciliation + ADRs
    └──> TASK-009 dynamic/generated compatibility
             │
             v
TASK-010 reconcile matrix + run gates + reviews
             │
             v
TASK-011 open PR and stop for maintainer review
```

TASK-007, TASK-008, and TASK-009 own separate documentation files and may proceed independently
after the red contract exists. This execution uses one worktree and applies them sequentially to
avoid needless coordination overhead.

## Safety and promotion gates

| Gate               | Pass condition                                                                                        | Failure action                                                   |
| ------------------ | ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Specification      | All issue #289 artifacts validate and composite reviews have no unresolved medium/high finding.       | Return to `/specify`; do not implement.                          |
| Scope              | Diff touches only specs, plans, architecture docs, tests, and review artifacts.                       | Remove the behavior change or split it to its owning ticket.     |
| Compatibility      | Existing tests and module validation pass unchanged.                                                  | Stop; architecture work cannot require current behavior changes. |
| Decision integrity | TypeSpec remains unpromoted, Avro remains current, and external ADR statuses are represented exactly. | Correct the record; do not weaken the source decision.           |
| Review             | Code review and gap analysis pass with all blocking findings fixed.                                   | Fix and rerun.                                                   |
| Merge              | Named Quoin/Quire maintainers approve the normative record.                                           | Leave PR open; do not merge.                                     |

## Definition of done

- TC-1125..TC-1154 pass with exact trace tags and matrix rows marked covered.
- TC-1155 has durable named-maintainer disposition in SR-058 and PR #311's merge record.
- Every local architecture link resolves and every external decision has repository, path, status,
  and revision/date.
- No behavior, manifest, schema, generated package, publication, persistence, or migration file changes.
- Direct repository validation passes, any canonical-preflight or absolute-coverage baseline
  limitation is recorded, review artifacts are committed, and the PR is ready for maintainer
  decision without additional implementation work.
