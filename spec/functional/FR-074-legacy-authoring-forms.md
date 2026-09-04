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
legacy form, Quire SHALL accept it at `warning` severity with a declared
migration, so that no corpus repository is edited by this campaign and no
current artifact becomes invalid.

## Rationale

Ticket #293 mapping (e) and the merge gate: advisory-first, normalize before
enforce, and human promotion for enforcing releases. The corpus is measured, not
rewritten.

## Behavior

- Quire SHALL recognise a bullet-list Properties section (each item `- <name>: <type>` with optional ` — <note>`) as legacy form `bullet-list`.
- Quire SHALL recognise a table under `## Properties` whose header is not the typed header as legacy form `free-column-table`.
- If a Properties section mixes a bullet list and a table, then Quire SHALL name the form of the first block.
- A legacy form SHALL yield one `warning` per artifact with code `semantic.legacy-properties-form`, the form name, the locus, and the migration target (`typed-table`).
- A legacy form SHALL still extract `properties` as the untyped section body exactly as today.
- Where the module manifest sets `semantic.legacy_forms: error`, Quire SHALL promote the warning to an error.
- The default for `semantic.legacy_forms` SHALL be `warning`.
- If a manifest sets `semantic.legacy_forms: error` without `semantic.sweep_report` naming a shipped file that validates against the sweep-report schema and records the same `package` and module `version`, then Quoin SHALL reject the manifest at install naming the missing or mismatched report.
- The sweep-report schema SHALL be `{ package, version, generatedAt, corpus: [{ repository, revision }], counts: { artifacts, legacy: { bullet-list, free-column-table } } }`, produced by `quoin semantic sweep` (`agent-ix/quoin#291` runs it over the corpus).
- Quoin SHALL document the migration once in the module's authoring pack (`quoin write`) with a before/after example.
- Quoin SHALL NOT edit, rewrite, or auto-migrate any artifact in a corpus repository.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-074-CON-1 | Legacy-form detection SHALL leave the extracted `properties` value unchanged and, by default, advisory. | Compatibility | Existing-fixture suite |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-074-AC-1 | Quoin's pinned copy of the unmodified config-service FR-006 (free-column table `Column \| Type \| Constraints`) has an expected single `semantic.legacy-properties-form` warning naming `free-column-table` and the locus, and its `properties` extraction fixture is byte-identical to today's. | Test |
| FR-074-AC-2 | A bullet-list Properties section has the expected warning with form `bullet-list`; a mixed section names the first block's form. | Test |
| FR-074-AC-3 | With `semantic.legacy_forms: error` and a valid `sweep_report`, the artifact's expected diagnostic is an error; with `legacy_forms: error` and no report, or a report for another package or version, the manifest is rejected at install naming the report. | Test |
| FR-074-AC-4 | The authoring pack shows the migration example once. | Test |
| FR-074-AC-5 | A sweep report produced by `quoin semantic sweep` over the fixture corpus validates against the sweep-report schema and counts each legacy form. | Test |

## Dependencies

- **Upstream**: [FR-071](./FR-071-typed-properties-mapping.md), [FR-073](./FR-073-data-schema-by-path-and-digest.md)
- **Downstream**: `agent-ix/quoin#291` (measurement sweep), `agent-ix/quoin#287` (catalog locks), `agent-ix/quire-rs#388`
