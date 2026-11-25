---
id: FR-229
title: "The dossier software description identifies the software and its recorded purpose"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-024"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
---

# FR-229: The dossier software description identifies the software and its recorded purpose

## Description

When `quoin dossier generate` selects `software-description`, Quoin SHALL write a software description from the selected repository's person-authored `software_description` records.

The document SHALL identify the software by the stable identifiers and names stated in those records and describe only their recorded purpose, functions, operating environment, and interfaces.

The document SHALL identify each source record by its type and artifact id.

The document SHALL carry the running Quoin producer identity required by [FR-191](./FR-191-dossier-command.md) and the shared `DossierDocument` contract in spec-artifacts-process FR-018.

The generator SHALL NOT infer product purpose, intended use, safety class, criticality, risk scores, or acceptability.

## Inputs

- The selected scope and module roots used by the other dossier generators.
- Person-authored `software_description` records in the selected scope. Each record has the artifact `id` and `type`, a stable `software_id`, a `name`, and optional `purpose`, `functions`, `operating_environment`, and `interfaces` fields.
- Each value in `functions` and `interfaces` names a source artifact id and type when it is backed by another selected record.

## Outputs

- `dossier/DOS-SW.md`, with a `Software description` title, a `Sources` section containing each source record's artifact id and type, and a final `Gaps` section.
- Each stated value is accompanied by the source record id from which it was read. Missing values are represented as gaps and are not replaced with defaults.

## Behavior

- The generator SHALL include every selected `software_description` record.
- The generator SHALL include only functions and interfaces stated in the selected record or its explicitly referenced source records.
- The generator SHALL preserve authored values and source record ids without assigning or deriving a classification or evaluation.
- When a source record is unreadable or invalid, the command SHALL report a source failure under FR-191.
- When a source record is unreadable or invalid, the command SHALL NOT write a partial document.
- When no selected record identifies software, the document SHALL contain a `no_software_description_source` gap.
- When no selected record identifies software, the document SHALL be incomplete.
- The document SHALL identify the producing tool by the exact `producer_identity` required by FR-191.
- The document SHALL identify source records by their stable ids and artifact types.
- The document SHALL NOT claim a source revision, commit, digest, or provenance that its sources do not provide.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-229-AC-1 | Given selected valid product and software records, `software-description` emits `DOS-SW` with the recorded identifiers, names, purpose, functions, operating environment and interfaces, and lists each contributing source id and type. | Test |
| FR-229-AC-2 | Given a missing optional value or no software-identifying record, `DOS-SW` preserves stated values, emits the corresponding gap, and invents no value. | Test |
| FR-229-AC-3 | Given an unreadable selected source record, generation reports the source failure and leaves existing output files unchanged. | Test |
| FR-229-AC-4 | Given two generations over the same records, `DOS-SW` is byte-identical and carries the exact producer name and version specified by FR-191 and FR-018. | Test |

## Dependencies

- **Upstream**: [US-024](../usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md), [FR-191](./FR-191-dossier-command.md), and selected authored product and software records.
- **Downstream**: [FR-191](./FR-191-dossier-command.md) registers and orders this document kind.
