---
id: FR-063
title: "Change assurance record"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quoin/US-017"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/ix-flow/FR-013"
    type: "requires"
  - target: "ix://agent-ix/ix-flow/FR-018"
    type: "requires"
---

# FR-063: Change assurance record

## Description

A `ChangeAssuranceRecord` SHALL preserve one reviewable definition of success,
its source and impact premises, its proof obligations, and its disclosed
unknowns as an engine-independent semantic record. The record SHALL use the
versioned JSON Schema below.

Approval is not a field added after the record is written. An ix-flow decision
event names the record by `record_id` and `revision`, so the record and the
decision are joined by identity and no hash is involved. A record revision is
immutable once written (a new revision is a new file, never an edit), and git
history is the tamper evidence. A record qualifies as approved only when
FR-065 finds a matching `approved` event.

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/change-assurance-record-v1.schema.json",
  "title": "ChangeAssuranceRecordV1",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "record_id",
    "revision",
    "subject",
    "source_connections",
    "impact_snapshot",
    "definition",
    "review_workflow"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "record_type": { "const": "change_assurance" },
    "record_id": { "$ref": "#/$defs/identity" },
    "revision": { "type": "integer", "minimum": 1 },
    "subject": {
      "type": "object",
      "required": ["repository", "base_revision", "scope"],
      "properties": {
        "repository": { "type": "string", "minLength": 1 },
        "base_revision": { "type": "string", "minLength": 1 },
        "scope": {
          "type": "array",
          "minItems": 1,
          "uniqueItems": true,
          "items": { "type": "string", "minLength": 1 }
        }
      },
      "additionalProperties": false
    },
    "source_connections": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/$defs/source_connection" }
    },
    "impact_snapshot": {
      "type": "object",
      "required": ["identity", "revision", "completeness", "truncated", "gaps"],
      "properties": {
        "identity": { "$ref": "#/$defs/identity" },
        "revision": { "type": "string", "minLength": 1 },
        "completeness": {
          "type": "string",
          "enum": ["complete", "incomplete"]
        },
        "truncated": { "type": "boolean" },
        "gaps": {
          "type": "array",
          "items": { "type": "string", "minLength": 1 }
        }
      },
      "additionalProperties": false
    },
    "definition": {
      "type": "object",
      "required": [
        "requirements",
        "preservation_constraints",
        "proof_obligations",
        "unknowns"
      ],
      "properties": {
        "requirements": {
          "type": "array",
          "minItems": 1,
          "items": { "$ref": "#/$defs/requirement" }
        },
        "preservation_constraints": {
          "type": "array",
          "items": { "$ref": "#/$defs/preservation_constraint" }
        },
        "proof_obligations": {
          "type": "array",
          "minItems": 1,
          "items": { "$ref": "#/$defs/proof_obligation" }
        },
        "unknowns": {
          "type": "array",
          "items": { "$ref": "#/$defs/unknown" }
        }
      },
      "additionalProperties": false
    },
    "review_workflow": {
      "type": "object",
      "required": ["run_id", "decision_event_kind"],
      "properties": {
        "run_id": { "$ref": "#/$defs/identity" },
        "decision_event_kind": {
          "const": "change_assurance.review_decided"
        }
      },
      "additionalProperties": false
    }
  },
  "$defs": {
    "identity": {
      "type": "string",
      "pattern": "^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$"
    },
    "source_connection": {
      "type": "object",
      "required": ["source_id", "kind", "revision"],
      "properties": {
        "source_id": { "$ref": "#/$defs/identity" },
        "kind": {
          "type": "string",
          "enum": [
            "requirement",
            "test",
            "api",
            "architecture",
            "impact_evidence",
            "recovery_evidence",
            "other"
          ]
        },
        "revision": { "type": "string", "minLength": 1 }
      },
      "additionalProperties": false
    },
    "requirement": {
      "type": "object",
      "required": ["id", "statement", "source_ids"],
      "properties": {
        "id": { "$ref": "#/$defs/identity" },
        "statement": { "type": "string", "minLength": 1 },
        "source_ids": {
          "type": "array",
          "minItems": 1,
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/identity" }
        }
      },
      "additionalProperties": false
    },
    "preservation_constraint": {
      "type": "object",
      "required": ["id", "statement", "source_ids"],
      "properties": {
        "id": { "$ref": "#/$defs/identity" },
        "statement": { "type": "string", "minLength": 1 },
        "source_ids": {
          "type": "array",
          "minItems": 1,
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/identity" }
        }
      },
      "additionalProperties": false
    },
    "command": {
      "type": "object",
      "required": ["argv", "working_directory"],
      "properties": {
        "argv": {
          "type": "array",
          "minItems": 1,
          "items": { "type": "string" }
        },
        "working_directory": { "type": "string", "minLength": 1 }
      },
      "additionalProperties": false
    },
    "proof_obligation": {
      "type": "object",
      "required": [
        "proof_id",
        "statement",
        "obligation_ids",
        "evidence_kind",
        "command",
        "tool_identity",
        "configuration_ref"
      ],
      "properties": {
        "proof_id": { "$ref": "#/$defs/identity" },
        "statement": { "type": "string", "minLength": 1 },
        "obligation_ids": {
          "type": "array",
          "minItems": 1,
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/identity" }
        },
        "evidence_kind": { "$ref": "#/$defs/identity" },
        "command": { "$ref": "#/$defs/command" },
        "tool_identity": { "type": "string", "minLength": 1 },
        "configuration_ref": { "type": "string", "minLength": 1 }
      },
      "additionalProperties": false
    },
    "unknown": {
      "type": "object",
      "required": ["id", "statement", "disposition", "owner"],
      "properties": {
        "id": { "$ref": "#/$defs/identity" },
        "statement": { "type": "string", "minLength": 1 },
        "disposition": {
          "type": "string",
          "enum": ["open", "accepted", "deferred", "resolved"]
        },
        "owner": { "type": "string", "minLength": 1 },
        "resolution": { "type": "string", "minLength": 1 }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

Every array SHALL be sorted by its identity key before the record is written. A meaningful
empty `preservation_constraints` or `unknowns` array SHALL remain present.
Duplicate requirement, constraint, proof, unknown, or source identities SHALL
be refused rather than deduplicated. A `resolved` unknown SHALL carry a
non-empty `resolution`; a non-resolved unknown SHALL NOT carry one.

Each command's `working_directory` SHALL be a normalized repository-relative
POSIX path.

Each `argv` SHALL contain literal arguments without shell expansion.

## Record identity and lineage

`record_id` is assigned by the caller at revision 1 (a UUID or a caller-chosen
`identity`) and never derived from content. A record is addressed by the pair
`(record_id, revision)` and stored at `records/<record_id>/<revision>.json`.

The serialized boundary SHALL receive the exact UTF-8 JSON bytes, not an
already parsed host-language object. Quoin SHALL parse those bytes with a
duplicate-preserving JSON decoder so duplicate object names, invalid Unicode,
non-I-JSON numbers, and a byte-order mark remain observable and are refused
before any normalized value is constructed. Output uses one deterministic
serialization (RFC 8785 key and number rules), so two runs over the same input
write the same bytes; that is checked by comparing the two outputs byte for
byte, never by hashing.

Revision 1 has no predecessor. Revision N greater than 1 requires retained
revision N-1 of the same `record_id`. Writing a successor SHALL write a new
file and SHALL NOT replace or mutate its predecessor.

## ix-flow decision contract

The matching ix-flow event SHALL have kind
`change_assurance.review_decided`, `actor.kind: human`, and a payload containing
exactly `schema_version: 1`, `record_id`, `revision`,
`decision`, and optional `note`.

The event's `decision` SHALL be `approved`, `rejected`, or `revise`.

The event's run id SHALL equal `review_workflow.run_id`.

The event's record id and revision SHALL equal the record's id and revision.

The event's actor label is recorded attribution only and does not prove a
person's identity, authority, possession of a key, or non-repudiation.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-063-CON-1 | Writing and verification SHALL perform no Git, workflow, proof, network, or identity-provider operation. | Architecture | Inspection |
| FR-063-CON-2 | The store SHALL preserve every earlier revision and every rejected or revise-requested revision by `(record_id, revision)`. | Integrity | Test |
| FR-063-CON-3 | Documentation and output SHALL make no identity, authorization, signature, authenticity, or non-repudiation claim. | Responsibility | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-063-AC-1 | Schema v1 requires the record identity and revision, repository and base revision, non-empty scope and sources, impact snapshot, complete definition collections, and review-workflow binding; undeclared fields (including any digest member) are refused. | Test (TC-1261) |
| FR-063-AC-2 | Requirements and proof obligations are non-empty; meaningful empty preservation and unknown arrays remain present; duplicate identities are refused before the record is written. | Property (TC-1262) |
| FR-063-AC-3 | Every requirement retains its reviewed statement, and every proof fixes its owning evidence obligations, literal argv, repository-relative cwd, evidence kind, tool identity, and configuration reference. | Test (TC-1263) |
| FR-063-AC-4 | An incomplete or truncated impact snapshot and each open, accepted, deferred, or resolved unknown remain explicit rather than becoming a completeness claim. | Test (TC-1264) |
| FR-063-AC-5 | Withdrawn: a record carries no digest, so no digest vector remains (id not reused). | Withdrawn (TC-1265) |
| FR-063-AC-6 | Withdrawn: there is no stored digest for a leaf change to invalidate; tampering is visible in git history (id not reused). | Withdrawn (TC-1266) |
| FR-063-AC-7 | Withdrawn: there is no digest form to refuse; duplicate names, lone surrogates, non-I-JSON numbers and a byte-order mark are refused under AC-1 and AC-2 (id not reused). | Withdrawn (TC-1267) |
| FR-063-AC-8 | Revision 1 has no predecessor; a successor requires retained revision N-1 of the same record id; a missing, skipped, or cross-record predecessor fails lineage verification. | Property (TC-1268) |
| FR-063-AC-9 | Writing a successor leaves the predecessor's path and bytes unchanged, and rejected or revise-requested records remain addressable by `(record_id, revision)` (CON-2). | Integration (TC-1269) |
| FR-063-AC-10 | Only an intact matching ix-flow human decision event identifies the exact reviewed revision; mismatched run, kind, actor kind, record id, or revision cannot approve it. | Integration (TC-1270) |
| FR-063-AC-11 | Static and golden-text checks prove writing a record runs nothing and describes actor labels only as recorded attribution, never identity, authority, signature, authenticity, or non-repudiation (CON-1, CON-3). | Analysis (TC-1271) |

## Dependencies

- **Upstream**: [US-017](../usecase/US-017-verify-change-assurance-evidence.md),
  [FR-030](./FR-030-evidence-store.md), and ix-flow FR-013/018 event integrity
  and history contracts.
- **Downstream**: [FR-064](./FR-064-proof-attestation.md) binds candidate proof
  evidence; [FR-065](./FR-065-change-assurance-verification.md) derives approval
  and verification outcomes.
