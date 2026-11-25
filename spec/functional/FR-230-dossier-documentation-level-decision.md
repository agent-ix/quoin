---
id: FR-230
title: "The dossier records the person-authored software documentation-level decision"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-024"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
---

# FR-230: The dossier records the person-authored software documentation-level decision

## Description

When `quoin dossier generate` selects `documentation-level-decision`, Quoin SHALL render the selected, person-authored documentation-level decision record as `DOS-DL`.

The document SHALL report the documentation level, decision maker, decision date, rationale, and source record ids exactly as stated by the author.

The document SHALL report absent decision fields as gaps.

Quoin SHALL NOT select, recommend, or infer a documentation level.

Quoin SHALL NOT assign a software safety class, risk score, or acceptability.

## Inputs

- The selected scope and module roots used by the other dossier generators.
- Zero or more person-authored `documentation_level_decision` records in the selected scope. Each record has artifact `id` and `type`, `software_id`, `level`, `decided_by`, `decided_on`, `rationale`, and a list of `source_records` containing artifact ids and types.

## Outputs

- `dossier/DOS-DL.md`, containing the authored level, decision maker, decision date, rationale, referenced source record ids, the running Quoin producer identity, and a final `Gaps` section.
- The `Sources` section identifies each decision record by its type and artifact id. Any source record ids named by the decision are retained verbatim.

## Behavior

- The generator SHALL select decision records only from the caller's explicit scope and module selection.
- The generator SHALL associate a decision with software only through the record's `software_id`.
- The generator SHALL render the authored level, actor, date, rationale and references without changing their values.
- The generator SHALL NOT present the rendered decision as a Quoin conclusion.
- When no decision record is selected, the document SHALL emit a `no_documentation_level_decision` gap.
- When a selected record omits the decision maker, decision date, level, rationale, or source references, the document SHALL identify each absent field in its `Gaps` section.
- When a selected record omits any decision field, the document SHALL not supply a value for that field.
- When multiple selected records make different decisions for the same software identity, the document SHALL render each record separately and report a `conflicting_documentation_level_decisions` gap.
- When multiple selected records make different decisions for the same software identity, the document SHALL NOT choose one.
- The document SHALL identify the producing tool using the producer identity required by FR-191 and FR-018.
- The document SHALL list the decision and referenced source record ids and their artifact types.
- The document SHALL NOT claim a source revision, commit, digest, or provenance that the sources do not provide.
- When a selected source record is unreadable or invalid, generation SHALL report a source failure under FR-191.
- When a selected source record is unreadable or invalid, generation SHALL NOT write a partial document.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-230-AC-1 | Given one complete person-authored decision record, `documentation-level-decision` emits `DOS-DL` with the exact level, decision maker, date, rationale and source ids, plus the decision record's id and type. | Test |
| FR-230-AC-2 | Given no decision record or a record missing one or more required decision fields, `DOS-DL` reports a gap for each absence and supplies no default or inferred value. | Test |
| FR-230-AC-3 | Given two conflicting decisions for the same software identity, `DOS-DL` preserves both records and reports the conflict without selecting a level. | Test |
| FR-230-AC-4 | Given a record that contains a safety class, risk score or acceptability decision, generation preserves only fields in the documentation-level decision contract and does not compute or alter those values. | Test |
| FR-230-AC-5 | Given two generations over the same selected records, `DOS-DL` is byte-identical and carries the exact producer name and version specified by FR-191 and FR-018. | Test |

## Dependencies

- **Upstream**: [US-024](../usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md), [FR-191](./FR-191-dossier-command.md), and the selected person-authored decision records.
- **Downstream**: [FR-191](./FR-191-dossier-command.md) registers and orders this document kind.
