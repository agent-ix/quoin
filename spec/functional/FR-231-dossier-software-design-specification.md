---
id: FR-231
title: "The dossier software design specification renders the selected software design records"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-024"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
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
- Selected authored architecture records: software-item `part` records, `port` records, `connection` records, allocations, and architecture decision records.
- Each included software item is identified by a stable artifact id and type. The set of design records for an item is limited to that item's explicit architecture relationships; no association is inferred from names or paths.

## Outputs

- `dossier/DOS-SDS.md`, with a section for each selected software item, the design facts and relationships read from source records, a `Sources` section listing source ids and artifact types, the running Quoin producer identity, and a final `Gaps` section.
- Design values retain their source record identity. A missing or unresolved reference is reported as a gap.

## Behavior

- The generator SHALL include only records and relationships selected by the caller's scope and module selection.
- The generator SHALL associate design records with software only through the explicit architecture relationships in the selected records.
- The generator SHALL preserve authored design values.
- The generator SHALL NOT infer a relationship, interface, allocation, verification result, or design property.
- When a selected relationship refers to a missing or wrong-kind record, the document SHALL report the unresolved reference.
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
| FR-231-AC-1 | Given a selected software item with authored design elements, interfaces, allocations and relationships, `software-design-specification` emits `DOS-SDS` with those facts and the source id and type for each record. | Test |
| FR-231-AC-2 | Given an absent design source or an unresolved or wrong-kind reference, `DOS-SDS` reports a gap and does not invent or render a valid relationship for it. | Test |
| FR-231-AC-3 | Given a design record with a safety class, risk score or acceptability value, the generator does not assign or change those values and makes no adequacy or compliance claim. | Test |
| FR-231-AC-4 | Given two generations over the same records and selection, `DOS-SDS` is byte-identical and carries the exact producer name and version specified by FR-191 and FR-018. | Test |

## Dependencies

- **Upstream**: [US-024](../usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md), [FR-191](./FR-191-dossier-command.md), and selected authored design records.
- **Downstream**: [FR-191](./FR-191-dossier-command.md) registers and orders this document kind.
