---
id: FR-232
title: "The dossier device description identifies the device from its authored records"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-024"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
---

# FR-232: The dossier device description identifies the device from its authored records

## Description

When `quoin dossier generate` selects `device-description`, Quoin SHALL render the selected person-authored `device_description` records as `DOS-DEV`.

The document SHALL report the device's stable identifier, name, recorded purpose, intended use, operating context, and software item identifiers when those values are present in the selected records.

The document SHALL retain each source record's type and artifact id.

Quoin SHALL NOT invent missing product facts, assign a software safety class, or decide risk acceptability.

## Inputs

- The selected scope and module roots used by the other dossier generators.
- Selected person-authored `device_description` records. Each record has artifact `id` and `type`, a stable `device_id`, a `name`, `intended_use`, and `operating_context` fields, and a list of `software_ids`.
- Each listed `software_id` refers to the stable `software_id` in a selected `software_description` record.

## Outputs

- `dossier/DOS-DEV.md`, containing the recorded device identity and product facts, a `Sources` section listing source record ids and artifact types, the running Quoin producer identity, and a final `Gaps` section.
- Missing device identity or product facts are listed as gaps and are not filled from unrelated records.

## Behavior

- The generator SHALL include only device records and explicitly linked software-item records in the caller's selected scope.
- The generator SHALL associate software with a device only through the device record's `software_ids` list.
- The generator SHALL preserve the authored identity, purpose, intended-use and operating-context values without deriving them from a software name, repository name, or other artifact.
- When a device record references a missing or wrong-kind software item, the document SHALL report the reference as unresolved.
- When a device record references a missing or wrong-kind software item, the document SHALL NOT present the device-to-software association as valid.
- When no selected record identifies a device, the document SHALL emit a `no_device_description_source` gap.
- The document SHALL identify the producing tool using the producer identity required by FR-191 and FR-018.
- The document SHALL list the ids and artifact types of the records actually read.
- The document SHALL NOT claim a source revision, commit, digest, or provenance that the sources do not provide.
- When a selected source record is unreadable or invalid, generation SHALL report a source failure under FR-191.
- When a selected source record is unreadable or invalid, generation SHALL NOT write a partial document.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-232-AC-1 | Given one device record and linked software-item records, `device-description` emits `DOS-DEV` with the recorded device identifier, name, purpose, intended use, operating context and linked software ids, and lists every source id and type. | Test |
| FR-232-AC-2 | Given one or more absent device facts or no device record, `DOS-DEV` reports the corresponding gap and does not fill the value from an unrelated record. | Test |
| FR-232-AC-3 | Given a missing or wrong-kind software-item reference, `DOS-DEV` reports the unresolved reference and does not claim a valid association. | Test |
| FR-232-AC-4 | Given two generations over the same records and selection, `DOS-DEV` is byte-identical and carries the exact producer name and version specified by FR-191 and FR-018. | Test |

## Dependencies

- **Upstream**: [US-024](../usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md), [FR-191](./FR-191-dossier-command.md), and selected person-authored device, product, and software-item records.
- **Downstream**: [FR-191](./FR-191-dossier-command.md) registers and orders this document kind.
