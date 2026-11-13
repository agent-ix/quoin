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
decision are joined by identity. A record carries no digest and no hash is
stored in it. A decision may additionally carry a claimed approval: a claim, written by the
event's writer, that a named person's signed git tag approved this exact record
digest, shaped as a projection of Engineering Assurance's `approval-verify` result
(its FR-041). Quoin retains the claim as data and never verifies a signature, so
the claim has the same standing as the actor label until someone re-runs
`approval-verify` with trust the relying party supplies (FR-068). A record revision is
immutable once written (a new revision is a new file, never an edit), and git
history is the tamper evidence. A record qualifies as approved only when
FR-065 finds a matching `approved` event.

## Properties

| Field              | Type       | Multiplicity | Constraints                  |
| ------------------ | ---------- | ------------ | ---------------------------- |
| schema_version     | Integer    | 1..1         |                              |
| record_type        | String     | 1..1         | enumValues: change_assurance |
| record_id          | String     | 1..1         | minLength: 1, maxLength: 256 |
| revision           | Integer    | 1..1         | min: 1                       |
| subject            | JsonObject | 1..1         |                              |
| source_connections | JsonObject | 1..*         |                              |
| impact_snapshot    | JsonObject | 1..1         |                              |
| definition         | JsonObject | 1..1         |                              |
| review_workflow    | JsonObject | 1..1         |                              |

The table declares the record's top-level fields and their cardinality. Which keys are required, which values may be null, and the cross-field rules are stated only by the schema below, which is the full contract.

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/change-assurance-record-v2.schema.json",
  "title": "ChangeAssuranceRecordV2",
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
    "schema_version": { "const": 2 },
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
      "required": ["run_id", "decision_event_kind", "signed_approval"],
      "properties": {
        "run_id": { "$ref": "#/$defs/identity" },
        "decision_event_kind": {
          "const": "change_assurance.review_decided"
        },
        "signed_approval": {
          "type": "object",
          "required": ["required", "signature_kinds", "approvers"],
          "properties": {
            "required": { "type": "boolean" },
            "signature_kinds": {
              "type": "array",
              "uniqueItems": true,
              "items": { "type": "string", "enum": ["ssh", "gitsign"] }
            },
            "approvers": {
              "type": "array",
              "uniqueItems": true,
              "items": { "type": "string", "minLength": 1 }
            }
          },
          "allOf": [
            {
              "if": { "properties": { "required": { "const": true } } },
              "then": { "properties": { "signature_kinds": { "minItems": 1 } } },
              "else": {
                "properties": {
                  "signature_kinds": { "maxItems": 0 },
                  "approvers": { "maxItems": 0 }
                }
              }
            }
          ],
          "additionalProperties": false
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
      "required": ["source_id", "kind", "revision", "path"],
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
        "revision": { "type": "string", "minLength": 1 },
        "path": {
          "type": "string",
          "pattern": "^(?!\\.\\.?(?:/|$))[^/\\\\\\u0000]+(?:/(?!\\.\\.?(?:/|$))[^/\\\\\\u0000]+)*$"
        }
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

Each source connection SHALL bind its `source_id` to the `path` of the artifact
it names, so a consumer reads the repository path from the record instead of
decoding it out of the identity. `source_id` stays an opaque identity and
carries no path convention. A `path` SHALL be a normalized repository-relative
POSIX path under the same rule as `working_directory`, except that it SHALL NOT
be `.`: it is not absolute, contains no backslash or NUL, has no trailing
slash, and has no empty, `.` or `..` segment. Two connections SHALL NOT share a
`path`; a duplicate is refused rather than deduplicated. A `path` names where
the artifact lives and nothing more: reading the bytes at the candidate
revision is the producer's responsibility, and the record holds no digest of
them.

Only the record body is evidence. Anything held outside it (a purpose, a
`derived_fields` map, a lookup map, or any other declaration metadata) is not
evidence, has no authority over the record, and is not read by verification. The
closed schema refuses such a member inside the record body at every depth, so
no record member can carry unreviewed metadata.

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
exactly `schema_version: 2`, `record_id`, `revision`,
`decision`, optional `note`, and optional `verified_approval`. The payload schema
is revised from version 1, which carried no `verified_approval`; the event set
stays closed and version 1 payloads are not read (the system is prerelease and
keeps no compatibility reader).

`verified_approval`, when present, SHALL be an object that projects the
`engineering-assurance.approval-verify-result/v1` result (Engineering Assurance
FR-041): it keeps `protocol`, the fields the verification rules need and `trust`, the trust input the writer used, kept as information only,
and omits the result's `signature`,
`binding` and `reason` members. It is closed:

| Member | Type | Presence |
| --- | --- | --- |
| protocol | exactly `engineering-assurance.approval-verify-result/v1` | always |
| status | `approved`, `wrong_record`, `untrusted_signer`, `meaning_missing`, `signed_bad`, `unsigned`, `unverifiable` | always |
| tag | non-empty string, the signed annotated tag name | always |
| record_id | identity | always |
| record_digest | non-empty string of at most 256 bytes | always |
| trust | object: `allowed_signers` (non-empty string path) and, when `approval-verify` was given one, `gitsign` (`certificate_identity`, `certificate_oidc_issuer`, `principal`, each a non-empty string) | always |
| approver | non-empty string, the principal the trust input names for the signer | exactly when status is `approved` |
| meaning | non-empty array of strings, the `Signed-off-meaning` values in order | exactly when status is `approved` |
| signature_kind | `ssh` or `gitsign` | exactly when status is `approved` |

An undeclared member, a member present outside its row, a `protocol` other than the
one above, or a `verified_approval` on a decision other than `approved` makes the
event a refused candidate, not an approval. Quoin does not interpret `meaning`,
compare `approver` to the event's actor, or decide who may approve.

**Standing of the claim.** Nothing binds the payload to a run of
`approval-verify`: the event's writer supplies it, and Quoin cannot re-verify a
signature. `verified_approval` is therefore the writer's claim, with the same
standing as the actor label, until `approval-verify` is re-run against the retained
`tag`, `record_id` and `record_digest` with trust the relying party supplies (FR-068). The name `verified_approval`
describes the shape of the claim, not a check Quoin made.

**Policy.** `review_workflow.signed_approval` is part of the reviewed record, so the
record digest covers it. It is the closed object `{required, signature_kinds,
approvers}` and may only narrow what is acceptable:

- `required: true` demands a `verified_approval` on the deciding event.
- `signature_kinds` lists the acceptable `signature_kind` values (`ssh`, `gitsign`),
  at least one when `required` is true.
- `approvers` lists the acceptable approver principals, empty meaning any principal
  the relying party's trust names.
- `required: false` takes `signature_kinds` and `approvers` empty. Any other
  combination is refused when the record is written.

**The record cannot establish trust.** The record names no allowed-signers file, no
repository revision to read one from, no gitsign identity or issuer, and no key. The
author of a change writes the record and can write anything into it, including a
signers file and a principal naming their own key, so any trust root taken from the
record or from a revision it names would let an author approve their own change as
someone else. The trust that makes an approver credible comes only from the relying
party (FR-068): its own allowed-signers file and gitsign identities, or a file read
at a ref it protects, such as the head of its target branch. The claim's `trust`
member records what the writer used and is informational only. Quoin checks a claim
only against the narrowing policy above. The record schema is revised to version 2 by
this member; version 1 records are not read (prerelease, no compatibility reader).

The record digest is `sha256:` followed by the lowercase hexadecimal SHA-256 of the
exact bytes of the file `records/<record_id>/<revision>.json`, which are the
deterministic serialization with no trailing newline. The reader hashes the bytes it
reads, with no newline, end-of-line or encoding normalization, and a repository
holding records SHALL mark `records/**` as `-text` so a checkout converts nothing.
It is computed by the
reader from those bytes (a local computation, not a Git, network or identity
operation) and is never stored in the record, so the record body stays digest-free.

The event's `decision` SHALL be `approved`, `rejected`, or `revise`.

The event's run id SHALL equal `review_workflow.run_id`.

The event's record id and revision SHALL equal the record's id and revision.

The event's actor label is recorded attribution only and does not prove a
person's identity, authority, possession of a key, or non-repudiation. A
`verified_approval` whose status is `approved` adds a claimed signer, with the same
standing, that Quoin retains and does not check.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-063-CON-1 | Writing and verification SHALL perform no Git, workflow, proof, network, or identity-provider operation. | Architecture | Inspection |
| FR-063-CON-2 | The store SHALL preserve every earlier revision and every rejected or revise-requested revision by `(record_id, revision)`. | Integrity | Test |
| FR-063-CON-3 | Documentation and output SHALL make no identity, authorization, signature, authenticity, or non-repudiation claim of Quoin's own; the only signer statement is a retained `verified_approval`, reported as a claim. | Responsibility | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-063-AC-1 | Schema v2 requires the record identity and revision, repository and base revision, non-empty scope and sources, impact snapshot, complete definition collections, and review-workflow binding with its `signed_approval` policy; undeclared fields (including any digest member) are refused. | Test |
| FR-063-AC-2 | Requirements and proof obligations are non-empty; meaningful empty preservation and unknown arrays remain present; duplicate identities are refused before the record is written. | Property |
| FR-063-AC-3 | Every requirement retains its reviewed statement, and every proof fixes its owning evidence obligations, literal argv, repository-relative cwd, evidence kind, tool identity, and configuration reference. | Test |
| FR-063-AC-4 | An incomplete or truncated impact snapshot and each open, accepted, deferred, or resolved unknown remain explicit rather than becoming a completeness claim. | Test |
| FR-063-AC-5 | Withdrawn: a record carries no digest, so no digest vector remains (id not reused). | Withdrawn |
| FR-063-AC-6 | Withdrawn: there is no stored digest for a leaf change to invalidate; tampering is visible in git history (id not reused). | Withdrawn |
| FR-063-AC-7 | Withdrawn: there is no digest form to refuse; duplicate names, lone surrogates, non-I-JSON numbers and a byte-order mark are refused under AC-1 and AC-2 (id not reused). | Withdrawn |
| FR-063-AC-8 | Revision 1 has no predecessor; a successor requires retained revision N-1 of the same record id; a missing, skipped, or cross-record predecessor fails lineage verification. | Property |
| FR-063-AC-9 | Writing a successor leaves the predecessor's path and bytes unchanged, and rejected or revise-requested records remain addressable by `(record_id, revision)` (CON-2). | Integration |
| FR-063-AC-10 | Only an intact matching ix-flow human decision event identifies the exact reviewed revision; mismatched run, kind, actor kind, record id, revision, or a payload `schema_version` other than 2 cannot approve it. | Integration |
| FR-063-AC-11 | Static and golden-text checks prove writing a record runs nothing and describes actor labels only as recorded attribution, never identity, authority, signature, authenticity, or non-repudiation, and that a signer appears only as a retained `verified_approval` described as a claim (CON-1, CON-3). | Analysis |
| FR-063-AC-12 | Every source connection requires a `path`, and the gate refuses an absolute path, a `..` or `.` segment, an empty segment, a backslash, a NUL, a trailing slash, the bare `.`, and a path shared by two connections; a valid path survives a write and re-read unchanged, and the published schema refuses the same malformed paths. | Test |
| FR-063-AC-13 | Only the record body is evidence: a `purpose`, `derived_fields` or lookup-map member placed inside the record body, at the top level or inside a source connection, is refused as undeclared. | Test |
| FR-063-AC-14 | A v2 payload with a well-formed `verified_approval` on an `approved` decision is a candidate; each of an undeclared member, a `protocol` other than the FR-041 result protocol, a missing `trust`, `approver`, `meaning` or `signature_kind` on a non-`approved` status, a missing one on `approved`, an empty `meaning`, and a `verified_approval` on `rejected` or `revise` is refused as a candidate. | Test |
| FR-063-AC-15 | The record digest of a stored revision equals `sha256:` plus the lowercase SHA-256 hex of its stored bytes, is identical across two writes of the same body, differs for a one-byte change, is computed without newline or end-of-line normalization, and the record body, schema and stored file contain no digest member. | Test |
| FR-063-AC-16 | A `signed_approval` with `required` true and no `signature_kinds`, with `required` false and any kind or approver, with a repeated or unknown kind, or with an empty or repeated approver, is refused when the record is written, and the published schema refuses the same; a record without `signed_approval` is refused as v2; changing `signed_approval` changes the record digest; and the schema has no member that names an allowed-signers file, a revision, a gitsign identity, an issuer or a key. | Test |

## Dependencies

- **Upstream**: [US-017](../usecase/US-017-verify-change-assurance-evidence.md),
  [FR-030](./FR-030-evidence-store.md), and ix-flow FR-013/018 event integrity
  and history contracts.
- **Downstream**: [FR-064](./FR-064-proof-attestation.md) binds candidate proof
  evidence; [FR-065](./FR-065-change-assurance-verification.md) derives approval
  and verification outcomes.
