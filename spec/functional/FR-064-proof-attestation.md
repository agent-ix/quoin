---
id: FR-064
title: "Candidate-bound proof attestation"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quoin/US-017"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-063"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
---

# FR-064: Candidate-bound proof attestation

## Description

A `ProofAttestation` SHALL transcribe one proof result and bind it to the exact
reviewed record, candidate revision, proof obligation, command, tool,
configuration, environment, and retained output. It states what a producer
reported; it does not decide whether the proof discharges the obligation.

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/proof-attestation-v1.schema.json",
  "title": "ProofAttestationV1",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "attestation_id",
    "digest",
    "record_digest",
    "candidate_revision",
    "proof_id",
    "command",
    "tool",
    "environment",
    "observed_at",
    "result",
    "retained_output"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "record_type": { "const": "proof_attestation" },
    "attestation_id": { "$ref": "#/$defs/identity" },
    "digest": { "$ref": "#/$defs/digest" },
    "record_digest": { "$ref": "#/$defs/digest" },
    "candidate_revision": { "type": "string", "minLength": 1 },
    "proof_id": { "$ref": "#/$defs/identity" },
    "command": { "$ref": "#/$defs/command" },
    "tool": {
      "type": "object",
      "required": ["identity", "version", "configuration_digest"],
      "properties": {
        "identity": { "type": "string", "minLength": 1 },
        "version": { "$ref": "#/$defs/immutable_version" },
        "configuration_digest": { "$ref": "#/$defs/digest" }
      },
      "additionalProperties": false
    },
    "environment": {
      "type": "object",
      "minProperties": 1,
      "additionalProperties": {
        "anyOf": [
          { "type": "string" },
          { "type": "number" },
          { "type": "boolean" },
          { "type": "null" }
        ]
      }
    },
    "observed_at": { "type": "string", "format": "date-time" },
    "result": {
      "type": "string",
      "enum": ["passed", "failed", "unavailable", "not_computed"]
    },
    "retained_output": {
      "type": "object",
      "required": ["media_type", "digest", "size_bytes"],
      "properties": {
        "media_type": { "type": "string", "minLength": 1 },
        "digest": { "$ref": "#/$defs/digest" },
        "size_bytes": { "type": "integer", "minimum": 0 }
      },
      "additionalProperties": false
    }
  },
  "$defs": {
    "identity": {
      "type": "string",
      "pattern": "^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$"
    },
    "digest": {
      "type": "string",
      "pattern": "^[a-f0-9]{64}$"
    },
    "immutable_version": {
      "type": "string",
      "pattern": "^(v?[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|[a-f0-9]{64})$"
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
    }
  },
  "additionalProperties": false
}
```

## Intake and integrity

The intake operation SHALL receive the attestation's exact UTF-8 JSON bytes plus
the exact retained output bytes. It SHALL use FR-063's duplicate-preserving raw
JSON boundary, recompute the output's 32-byte BLAKE3 lowercase-hex digest and
byte length before writing either artifact, and compute `digest` with FR-063's
RFC 8785/BLAKE3 procedure after removing only the top-level `digest` member.

The operation SHALL validate the attestation schema and both digests before an
atomic content-addressed write. It SHALL stage `attestation.json` and the output
bytes together in a new temporary directory, durably close both files, and
atomically rename that complete directory to the attestation digest. A crash or
error before the rename SHALL leave no visible attestation or output; recovery
SHALL discard incomplete temporary directories. Repeating identical intake
SHALL be byte-idempotent. An existing digest with different bytes SHALL produce
a collision error and preserve the existing directory and both artifacts.

`unavailable` and `not_computed` attestations SHALL retain producer diagnostics
as their output bytes. Completely missing evidence has no attestation and is
represented by FR-065 as `missing_attestation`. No result, including `passed`,
discharges a proof until FR-065 checks all bindings and FR-032 findings.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-064-CON-1 | Intake SHALL execute no proof command, tool, Git operation, workflow, or network request. | Architecture | Inspection |
| FR-064-CON-2 | Intake SHALL neither infer a candidate revision, proof id, command, tool/configuration field, result, environment, nor output digest. | Integrity | Test |
| FR-064-CON-3 | An attestation SHALL express no auditor verdict, approval, identity, authorization, or non-repudiation claim. | Responsibility | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-064-AC-1 | Schema v1 requires the attestation identity/digest, reviewed-record digest, candidate revision, proof id, exact argv/cwd, immutable tool identity/version/configuration, environment, timestamp, result, and retained-output media type/digest/size; undeclared fields are refused. | Test (TC-1272) |
| FR-064-AC-2 | Passed, failed, unavailable, and not-computed results remain four distinct producer states and none is converted into a verification outcome during intake. | Test (TC-1273) |
| FR-064-AC-3 | The exact retained bytes reproduce the declared BLAKE3 digest and size; changed, absent, or extra bytes leave neither a new attestation nor output artifact. | Property (TC-1274) |
| FR-064-AC-4 | The attestation digest follows the same pinned RFC 8785/BLAKE3 contract as FR-063, and changing each semantic field independently invalidates it. | Property (TC-1275) |
| FR-064-AC-5 | Each missing record, candidate, proof, argv/cwd, tool, configuration, environment, timestamp, result, or output field is refused independently rather than inferred. | Property (TC-1276) |
| FR-064-AC-6 | Repeated identical intake preserves byte identity; injected failures before the directory rename expose neither artifact and are recoverable; a same-digest/different-content collision preserves the first attestation and output. | Integration (TC-1277) |
| FR-064-AC-7 | An unavailable or not-computed producer result retains its diagnostic output, while entirely missing evidence creates no synthetic attestation. | Test (TC-1278) |
| FR-064-AC-8 | Existing FR-030 run evidence remains readable and unchanged; proof attestations occupy a distinct versioned store family. | Integration (TC-1279) |
| FR-064-AC-9 | Static boundaries and golden terminology prove intake runs nothing and reports only producer facts, never an audit, approval, identity, authorization, or non-repudiation conclusion (CON-1, CON-3). | Analysis (TC-1280) |

## Dependencies

- **Upstream**: [FR-063](./FR-063-change-assurance-record-integrity.md)
  defines the reviewed proof and shared digest procedure; [FR-030](./FR-030-evidence-store.md)
  defines the retained-evidence boundary.
- **Downstream**: [FR-065](./FR-065-change-assurance-verification.md)
  selects and audits attestations into a verification receipt.
