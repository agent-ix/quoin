---
id: FR-233
title: "Define the dossier product-source record contract"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-024"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/spec-objects-architecture/FR-007"
    type: "depends_on"
---

# FR-233: Define the dossier product-source record contract

## Description

Quoin SHALL read product-source JSON records only from `dossier/sources/` beneath the selected `--scope` root.

These JSON records SHALL use the closed record contract below; their `record_type` values are dossier input-record kinds, not Quire catalog object types.

Quoin SHALL validate each selected record against its record-kind schema before any dossier document is written.

## Inputs

Each file is UTF-8 JSON at `dossier/sources/<record_type>/<id>.json`. Every record SHALL carry `schema_version: 1`, a non-empty `id`, and one `record_type` from this closed set: `software_description`, `documentation_level_decision`, or `device_description`. `id` SHALL match `[A-Za-z0-9][A-Za-z0-9._-]*` and SHALL be unique within its record kind and selected scope. Every string value SHALL be non-empty; every array member SHALL be a non-empty string except for the documented `source_records` objects. A record with an unknown member, missing required member, invalid member type, or duplicate id is invalid.

### `software_description` record

| Member | Type | Required | Contract |
|---|---|---:|---|
| `schema_version` | integer | yes | Value is `1`. |
| `id` | string | yes | Stable identifier for this source record; unique among selected records of this kind. |
| `record_type` | string | yes | Value is `software_description`. |
| `software_id` | string | yes | Stable identity for the software item; unique among selected records of this kind. |
| `name` | string | yes | Non-empty authored name. |
| `purpose` | string | no | Authored purpose. |
| `functions` | array of strings | no | Authored software functions. |
| `operating_environment` | string | no | Authored operating environment. |
| `interfaces` | array of strings | no | Authored interface descriptions. |
| `architecture_part_ids` | array of strings | no | Artifact ids of architecture `part` records explicitly representing this software item. |
| `architecture_decision_ids` | array of strings | no | Artifact ids of architecture decision records explicitly associated with this software item. |

### `documentation_level_decision` record

| Member | Type | Required | Contract |
|---|---|---:|---|
| `schema_version` | integer | yes | Value is `1`. |
| `id` | string | yes | Stable identifier for this source record; unique among selected records of this kind. |
| `record_type` | string | yes | Value is `documentation_level_decision`. |
| `software_id` | string | yes | Stable software identity from one selected `software_description` record. |
| `level` | string | no | Person-authored documentation level; Quoin treats it as opaque text. |
| `decided_by` | string | no | Person identity as authored. |
| `decided_on` | string | no | Person-authored valid calendar date in `YYYY-MM-DD` form. |
| `rationale` | string | no | Person-authored rationale. |
| `source_records` | array of objects | no | Each object has exactly an `id` matching `[A-Za-z0-9][A-Za-z0-9._-]*` and a `record_type`; `record_type` is one of the three record kinds defined by this contract. Missing or empty is reported as unstated. |

### `device_description` record

| Member | Type | Required | Contract |
|---|---|---:|---|
| `schema_version` | integer | yes | Value is `1`. |
| `id` | string | yes | Stable identifier for this source record; unique among selected records of this kind. |
| `record_type` | string | yes | Value is `device_description`. |
| `device_id` | string | yes | Stable device identity; unique among selected records of this kind. |
| `name` | string | yes | Non-empty authored device name. |
| `purpose` | string | no | Authored product purpose. |
| `intended_use` | string | no | Person-authored intended use. |
| `operating_context` | string | no | Authored operating context. |
| `software_ids` | array of strings | no | Stable `software_id` values from selected `software_description` records. |

The schema for each record kind SHALL reject additional members. Required fields and member types above define structural validity. Each optional field may be absent; an empty array is structurally valid and is reported as unstated by the consuming dossier document. An absent optional field or empty array is reported as unstated by the consuming dossier document. An empty string, a string array containing an empty string, or a malformed JSON file is structurally invalid and SHALL be reported as a source failure naming its path and reason under FR-191.

## Outputs

- `software-description-source-v1.schema.json`, `documentation-level-decision-source-v1.schema.json`, and `device-description-source-v1.schema.json`, each a JSON Schema 2020-12 file for the matching record kind defined above.
- A source-validation result that distinguishes JSON parse failures, schema failures, and duplicate ids without coercing or repairing source values.

## Behavior

- Quoin SHALL read only files beneath `<scope>/dossier/sources/` whose path record-kind segment agrees with the JSON `record_type` member and whose filename stem agrees with the JSON `id` member.
- Quoin SHALL reject two selected records of the same record kind with the same `id` or two `software_description` records with the same `software_id`.
- Quoin SHALL reject two selected `device_description` records with the same `device_id`.
- Quoin SHALL validate source-reference member shapes without requiring target records to resolve during schema validation.
- The consuming dossier document SHALL report any `software_id` in a `documentation_level_decision` record that does not resolve to one selected `software_description` record using the `source_reference_unresolved` gap defined by FR-191.
- The consuming dossier document SHALL report any reference in `architecture_part_ids`, `architecture_decision_ids`, `software_ids`, or `source_records` that does not resolve to one selected artifact of the stated id and kind using the `source_reference_unresolved` gap defined by FR-191.
- Quoin SHALL preserve all person-authored values exactly.
- Quoin SHALL NOT assign a documentation level, software safety class, criticality, risk score, or acceptability.
- Quoin SHALL NOT infer or add a software-to-part or device-to-software association that no selected source record states.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-233-AC-1 | Each valid version-1 source record kind validates against its matching schema; a record with a missing required member, wrong member type, extra member, or path/`record_type`/`id` mismatch is rejected as a source failure naming its path. | Test |
| FR-233-AC-2 | Duplicate ids, duplicate `software_id` values, and duplicate `device_id` values are rejected within the selected scope; the same id in two different record kinds remains valid. | Test |
| FR-233-AC-3 | Each reference member validates its array and string shape without resolving its target; the consuming dossier document reports an unresolved or wrong-kind target as `source_reference_unresolved` with the source-record id, referenced id, and expected record kind or artifact type. | Test |
| FR-233-AC-4 | Valid records with different field ordering produce byte-identical normalized source-validation results, and all authored string values are preserved without trimming, inference, or substitution. | Test |
| FR-233-AC-5 | Quoin's three emitted source-record schemas match the member names, types, requiredness, record-type enum values, non-empty string rules, and additional-member rejection defined in this requirement, while the source reader enforces record-id and domain-identity uniqueness. | Test |

## Dependencies

- **Upstream**: [US-024](../usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md), and spec-objects-architecture [FR-007](ix://agent-ix/spec-objects-architecture/FR-007) for architecture `part` records.
- **Downstream**: [FR-229](./FR-229-dossier-software-description.md), [FR-230](./FR-230-dossier-documentation-level-decision.md), and [FR-232](./FR-232-dossier-device-description.md) consume these source records; [FR-231](./FR-231-dossier-software-design-specification.md) uses `software_description.architecture_part_ids` to select parts.
