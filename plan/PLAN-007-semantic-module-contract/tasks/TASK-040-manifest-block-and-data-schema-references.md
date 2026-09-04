---
id: TASK-040
title: "Manifest block and data_schema references"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-039"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-070"
    type: references
  - target: "ix://agent-ix/quoin/FR-073"
    type: references
  - target: "ix://agent-ix/quoin/TC-1336"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1337"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1338"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1339"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1340"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1341"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1360"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1361"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1362"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1363"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1364"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1365"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1366"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1383"
    type: verifies
---

# TASK-040: Manifest block and data_schema references

## Scope

Read and validate the `semantic` block and `data_schema` reference form at `quoin module install`, and surface the block in the authoring pack.

## Subtasks

- [x] `src/semantic/manifest.ts`: parse the block against the vendored schema; rejections for unknown key, undeclared export, unknown contract version, unknown target, malformed package, duplicate package (sorted root order) (TC-1338..1341, TC-1383).
- [x] `src/semantic/data-schema.ts`: reference-form resolution, digest over raw bytes, missing/non-JSON/`$id`-less/escape/symlink/ambiguous rejections, `$ref` resolution against the shipped bundle and vendored semantic-core bundle with version and cycle checks, inline-schema warning (TC-1360..1364).
- [x] Wire into `src/commands/module/install.ts`; offline test with network disabled (TC-1365); existing-fixture suite unchanged (TC-1366, TC-1336).
- [x] `src/write.ts`: report semantic-core version, package, and schema paths (TC-1337).

## Deliverables

- Install-time semantic validation.
- Authoring-pack semantic section.

## Notes

- Artifact-time diagnostics are quire-rs#388's; do not reimplement extraction here.
