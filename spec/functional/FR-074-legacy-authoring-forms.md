---
id: FR-074
title: "Legacy authoring forms and declared migration"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-071"
    type: "depends_on"
---

# FR-074: Legacy authoring forms and declared migration

## Description

When an artifact under a semantic module authors its Properties section in a
legacy form, validation SHALL accept it at `warning` severity with a declared
migration, so that no corpus repository is edited by this campaign and no
current artifact becomes invalid.

## Rationale

Ticket #293 mapping (e) and the merge gate: advisory-first, normalize before
enforce, and human promotion for enforcing releases. The corpus is measured, not
rewritten.

## Behavior

- The validator SHALL recognise a bullet-list Properties section (`- name: type — note`) as legacy form `bullet-list`.
- The validator SHALL recognise a table under `## Properties` whose header is not the typed header as legacy form `free-column-table`.
- A legacy form SHALL yield one `warning` per artifact with code `semantic.legacy-properties-form`, the form name, the locus, and the migration target (`typed-table`).
- A legacy form SHALL still extract `properties` as the untyped section body exactly as today.
- Where the module manifest sets `semantic.legacy_forms: error`, the validator SHALL promote the warning to an error.
- The default for `semantic.legacy_forms` SHALL be `warning`.
- If a manifest sets `semantic.legacy_forms: error` without a recorded advisory sweep report (`agent-ix/quoin#291`), then the loader SHALL reject the manifest.
- Quoin SHALL document the migration once in the module's authoring pack (`quoin write`) with a before/after example.
- Quoin SHALL NOT edit, rewrite, or auto-migrate any artifact in a corpus repository.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-074-CON-1 | Legacy-form detection SHALL leave the extracted `properties` value unchanged and, by default, advisory. | Compatibility | Existing-fixture suite |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-074-AC-1 | The unmodified config-service FR-006 (free-column table `Column | Type | Constraints`) validates with exactly one `semantic.legacy-properties-form` warning naming `free-column-table` and the locus, and its `properties` extraction is byte-identical to today's. | Test |
| FR-074-AC-2 | A bullet-list Properties section yields the warning with form `bullet-list`. | Test |
| FR-074-AC-3 | With `semantic.legacy_forms: error` the same artifact fails; without the recorded sweep report the manifest setting itself is rejected. | Test |
| FR-074-AC-4 | The authoring pack shows the migration example once. | Test |

## Dependencies

- **Upstream**: [FR-071](./FR-071-typed-properties-mapping.md), [FR-073](./FR-073-data-schema-by-path-and-digest.md)
- **Downstream**: `agent-ix/quoin#291` (measurement sweep), `agent-ix/quoin#287` (catalog locks)
