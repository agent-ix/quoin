---
id: FR-230
title: "The dossier records the person-authored software documentation-level decision"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-024"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-233"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
---

# FR-230: The dossier records the person-authored software documentation-level decision

## Description

When `quoin dossier generate` selects `documentation-level-decision`, Quoin SHALL render selected `documentation_level_decision` records valid under [FR-233](./FR-233-dossier-product-source-record-contract.md) as `DOS-DL`.

The document SHALL report the documentation level, decision maker, decision date, rationale, and source record ids exactly as stated by the author.

The document SHALL report absent decision fields as gaps.

Quoin SHALL NOT select, recommend, or infer a documentation level.

Quoin SHALL NOT assign a software safety class, risk score, or acceptability.

## Inputs

- The selected scope and module roots used by the other dossier generators.
- Zero or more `documentation_level_decision` source records from `dossier/sources/documentation_level_decision/`, as defined by [FR-233](./FR-233-dossier-product-source-record-contract.md).

## Outputs

- `dossier/DOS-DL.md`, containing the authored level, decision maker, decision date, rationale, referenced source record ids, the running Quoin producer identity, and a final `Gaps` section.
- The `Sources` section identifies each decision record by its `record_type` and id. Any source record ids named by the decision are retained verbatim.

## Behavior

- The generator SHALL select decision records only from the caller's explicit scope and module selection.
- The generator SHALL associate a decision with software only through the record's `software_id`.
- When the `software_id` does not resolve to a selected `software_description` record, the document SHALL emit a `source_reference_unresolved` gap naming the decision record, software id, and expected source kind.
- The generator SHALL render the authored level, actor, date, rationale and references without changing their values.
- The generator SHALL NOT present the rendered decision as a Quoin conclusion.
- When no decision record is selected, the document SHALL emit a `no_documentation_level_decision` gap.
- When a selected record omits the decision maker, decision date, level, or rationale, or omits or has an empty `source_records` array, the document SHALL identify each unstated field in its `Gaps` section. An empty string is an invalid source value under FR-233 and causes a source failure.
- When a selected record omits any decision field, the document SHALL not supply a value for that field.
- When `source_records` references an absent or wrong-kind record, the document SHALL emit a `source_reference_unresolved` gap naming the decision record, referenced id, and expected record kind.
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
| FR-230-AC-1 | Given one complete decision record, `documentation-level-decision` emits `DOS-DL` with the exact level, decision maker, date, rationale and source ids, plus the decision record's id and `record_type`. | Test |
| FR-230-AC-2 | Given a decision record that omits any optional decision field or has an empty `source_records` array, `DOS-DL` emits one `documentation_level_decision_field_unstated` gap for each unstated field; an empty string instead causes a source failure under FR-233. | Test |
| FR-230-AC-3 | Given two conflicting decisions for the same software identity, `DOS-DL` preserves both records and reports the conflict without selecting a level. | Test |
| FR-230-AC-4 | Given a record that contains a safety class, risk score or acceptability decision, generation preserves only fields in the documentation-level decision contract and does not compute or alter those values. | Test |
| FR-230-AC-5 | Given two generations over the same selected records, `DOS-DL` is byte-identical and carries the exact producer name and version specified by FR-191 and FR-018. | Test |

## Dependencies

- **Upstream**: [US-024](../usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md), [FR-191](./FR-191-dossier-command.md), and [FR-233](./FR-233-dossier-product-source-record-contract.md).
- **Downstream**: [FR-191](./FR-191-dossier-command.md) registers and orders this document kind.
