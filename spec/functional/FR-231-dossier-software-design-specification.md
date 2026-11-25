---
id: FR-231
title: "The dossier software design specification renders the selected software design records"
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
  - target: "ix://agent-ix/spec-objects-architecture/FR-007"
    type: "depends_on"
---

# FR-231: The dossier software design specification renders the selected software design records

## Description

When `quoin dossier generate` selects `software-design-specification`, Quoin SHALL render the selected authored software design records as `DOS-SDS`.

The document SHALL identify each software item and show only recorded design elements, interfaces, allocations, and relationships within the selected scope.

The document SHALL retain the artifact id and type of each source record.

The document SHALL NOT claim that a design satisfies a safety or regulatory obligation or has passed verification.

The document SHALL NOT assign a software safety class, risk score, or acceptability.

## Inputs

- The selected scope and module roots used by the other dossier generators.
- Each selected `software_description` record under FR-233 creates exactly one software-item section, keyed by its `software_id`.
- The record's `architecture_part_ids` explicitly identifies its architecture `part` records. The selected architecture records also include `port`, `connection`, and `allocation` records whose declared owners or endpoints refer to those selected parts, plus architecture decision records whose ids appear in the software record's `architecture_decision_ids`.
- Part, port, connection, allocation, and architecture decision record shapes follow spec-objects-architecture FR-007 and Quoin's existing FR-193 architecture source contract.

## Outputs

- `dossier/DOS-SDS.md`, with a section for each selected software item, the design facts and relationships read from source records, a `Sources` section listing source ids and artifact types, the running Quoin producer identity, and a final `Gaps` section.
- Design values retain their source record identity. A missing or unresolved reference is reported as a gap.

## Behavior

- The generator SHALL include only records and relationships selected by the caller's scope and module selection.
- The generator SHALL emit one software-item section for each selected `software_description` record.
- The generator SHALL associate parts with a software item only when their ids occur in that record's `architecture_part_ids` list.
- The generator SHALL associate architecture decisions with a software item only when their ids occur in that record's `architecture_decision_ids` list.
- The generator SHALL associate a port, connection, or allocation with a software item only through the selected part ids and the relationships defined by spec-objects-architecture FR-007.
- The generator SHALL preserve authored design values.
- The generator SHALL NOT infer a relationship, interface, allocation, verification result, or design property.
- When an explicit part or decision id, or a selected relationship, refers to a missing or wrong-kind record, the document SHALL report a `source_reference_unresolved` gap naming the source record, referenced id, and expected artifact type.
- When a selected relationship refers to a missing or wrong-kind record, the document SHALL NOT render it as a valid design relationship.
- When no selected software design record exists, the document SHALL emit a `no_software_design_source` gap.
- The document SHALL identify the producing tool using the producer identity required by FR-191 and FR-018.
- The document SHALL list the ids and artifact types of the design records actually read.
- The document SHALL NOT claim a source revision, commit, digest, or provenance that the sources do not provide.
- When a selected source record is unreadable or invalid, generation SHALL report a source failure under FR-191.
- When a selected source record is unreadable or invalid, generation SHALL NOT write a partial document.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-231-AC-1 | Given a selected `software_description` record with explicit `architecture_part_ids` and `architecture_decision_ids`, `software-design-specification` emits one `DOS-SDS` section for its `software_id` and includes only those parts and decisions and architecture records related to those parts. | Test |
| FR-231-AC-2 | Given an absent design source or an unresolved or wrong-kind reference, `DOS-SDS` reports a gap and does not invent or render a valid relationship for it. | Test |
| FR-231-AC-3 | Given a design record with a safety class, risk score or acceptability value, the generator does not assign or change those values and makes no adequacy or compliance claim. | Test |
| FR-231-AC-4 | Given two generations over the same records and selection, `DOS-SDS` is byte-identical and carries the exact producer name and version specified by FR-191 and FR-018. | Test |

## Dependencies

- **Upstream**: [US-024](../usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md), [FR-191](./FR-191-dossier-command.md), [FR-233](./FR-233-dossier-product-source-record-contract.md), spec-objects-architecture [FR-007](ix://agent-ix/spec-objects-architecture/FR-007), and [FR-193](./FR-193-dossier-architecture-content.md).
- **Downstream**: [FR-191](./FR-191-dossier-command.md) registers and orders this document kind.
