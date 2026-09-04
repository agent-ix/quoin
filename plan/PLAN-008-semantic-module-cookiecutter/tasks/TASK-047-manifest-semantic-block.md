---
id: TASK-047
title: "Rendered manifest semantic block and digests"
type: Task
status: todo
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-046"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-078"
    type: references
  - target: "ix://agent-ix/quoin/TC-1410"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1417"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1419"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1458"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1459"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1468"
    type: verifies
---

# TASK-047: Rendered manifest semantic block and digests

## Scope

Render the module manifest carrying the `semantic` block FR-070 admits and a
`{schema, digest}` reference for every exported type, with the digests written by
the emit driver rather than authored.

## Subtasks

- [ ] Render `semantic` with `contract_version`, `semantic_core`, `package`, `exports`, `imports`, `targets`, `mappings`, `compatibility_posture: additive`, `legacy_forms: warning`, and no `sweep_report` (TC-1417).
- [ ] Render `artifact_types` for the artifact variant, `object_types` for the object variant, and both for mixed, each with `data_schema: {schema, digest}` and `body_extraction` locators.
- [ ] Rewrite the `digest:` lines textually from the emitted bytes so comments and YAML anchors survive regeneration (TC-1419).
- [ ] Assert the rendered `exports` and the declared type names are the same set, and that no type name appears twice (TC-1468, TC-1458).
- [ ] Map each imported package identity to its exact version, and refuse two entries naming the same identity (TC-1459).
- [ ] Validate the rendered manifest against Quoin's vendored module-manifest schema (TC-1417) and assert each digest equals the SHA-256 of the file it names (TC-1410).

## Deliverables

- A rendered manifest that declares the contract from its first commit, with no `{type: object}` placeholder anywhere.
