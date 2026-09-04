---
id: TASK-039
title: "Schema ownership and vendoring"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-038"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-070"
    type: references
  - target: "ix://agent-ix/quoin/FR-073"
    type: references
  - target: "ix://agent-ix/quoin/TC-1342"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1343"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1385"
    type: verifies
---

# TASK-039: Schema ownership and vendoring

## Scope

Land the `semantic` block in filament-core-service's module-manifest schema (agent-ix/filament-core-service#21) and vendor that schema plus the semantic-core 0.1.0 JSON Schema bundle into quoin with recorded provenance.

## Subtasks

- [x] Open the filament-core-service PR for #21: optional `semantic` object with the ten FR-070 keys, `data_schema` reference form, no new required key; merge it.
- [x] Add `scripts/refresh-manifest-schema.mjs` and `scripts/refresh-semantic-core-schemas.mjs` modelled on `refresh-quire-schemas.mjs`; vendor into `src/semantic/schemas/` with `src/semantic/contract.ts` recording repository, revision, path, sha256 (TC-1343, TC-1385).
- [x] Schema-diff test asserting `required` arrays unchanged versus the pre-#21 copy (TC-1342).

## Deliverables

- Merged filament-core-service#21.
- Vendored schemas with provenance and refresh scripts.

## Notes

- Cross-repo delivery: the filament-core-service change is part of landing this ticket.
- Vendored bundle digest must equal filament-core-data `packages/semantic-core/generated/toolchain.json` at the recorded revision.
