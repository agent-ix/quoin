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
configuration, environment, and retained output. Schema version 2 SHALL carry
a content commitment over the attestation metadata and exact retained output
bytes. The commitment is owned by this attestation contract and is verified by
FR-065. It states what a producer reported; it does not decide whether the
proof discharges the obligation.

Every attestation SHALL carry a `strength` naming the kind of support behind it
(FR-022, Engineering Assurance's `ClaimStrength`, PLAT-972): one of `proven`,
`bounded-checked`, `tested`, or `observed`, as declared by the producer — for
example, a bounded Kani `Proved` result attests `bounded-checked`. `strength` is
imported from Engineering Assurance's own vocabulary — this schema declares no
local enum for it — and, because the type itself implements neither `Ord` nor
`PartialOrd`, no report SHALL rank, compare, or aggregate one attestation's
strength against another's.

## Properties

| Field              | Type       | Multiplicity | Constraints                                           |
| ------------------ | ---------- | ------------ | ----------------------------------------------------- |
| schema_version     | Integer    | 1..1         |                                                       |
| record_type        | String     | 1..1         | enumValues: proof_attestation                         |
| attestation_id     | String     | 1..1         | minLength: 1, maxLength: 256                          |
| record_id          | String     | 1..1         | minLength: 1, maxLength: 256                          |
| record_revision    | Integer    | 1..1         | min: 1                                                |
| candidate_revision | String     | 1..1         | minLength: 1                                          |
| proof_id           | String     | 1..1         | minLength: 1, maxLength: 256                          |
| command            | JsonObject | 1..1         |                                                       |
| tool               | JsonObject | 1..1         |                                                       |
| environment        | JsonObject | 1..1         |                                                       |
| observed_at        | Timestamp  | 1..1         |                                                       |
| result             | String     | 1..1         | enumValues: passed\|failed\|unavailable\|not_computed |
| strength           | String     | 1..1         | enumValues: proven\|bounded-checked\|tested\|observed |
| retained_output    | JsonObject | 1..1         |                                                       |
| content_commitment | JsonObject | 1..1         |                                                       |

The table declares the record's top-level fields and their cardinality. Which keys are required, which values may be null, and the cross-field rules are stated only by the schema below, which is the full contract.

## Schema

The schema below is schema v2, the only schema. Pre-release, no earlier schema is read.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/proof-attestation-v2.schema.json",
  "title": "ProofAttestationV2",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "attestation_id",
    "record_id",
    "record_revision",
    "candidate_revision",
    "proof_id",
    "command",
    "tool",
    "environment",
    "observed_at",
    "result",
    "strength",
    "retained_output",
    "content_commitment"
  ],
  "properties": {
    "schema_version": { "const": 2 },
    "record_type": { "const": "proof_attestation" },
    "attestation_id": { "$ref": "#/$defs/identity" },
    "record_id": { "$ref": "#/$defs/identity" },
    "record_revision": { "type": "integer", "minimum": 1 },
    "candidate_revision": { "type": "string", "minLength": 1 },
    "proof_id": { "$ref": "#/$defs/identity" },
    "command": { "$ref": "#/$defs/command" },
    "tool": {
      "type": "object",
      "required": ["identity", "version", "configuration_ref"],
      "properties": {
        "identity": { "type": "string", "minLength": 1 },
        "version": { "$ref": "#/$defs/immutable_version" },
        "configuration_ref": { "type": "string", "minLength": 1 }
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
    "strength": {
      "type": "string",
      "enum": ["proven", "bounded-checked", "tested", "observed"]
    },
    "retained_output": {
      "type": "object",
      "required": ["media_type", "size_bytes"],
      "properties": {
        "media_type": { "type": "string", "minLength": 1 },
        "size_bytes": { "type": "integer", "minimum": 0 }
      },
      "additionalProperties": false
    },
    "content_commitment": {
      "type": "object",
      "required": ["algorithm", "digest"],
      "properties": {
        "algorithm": { "const": "sha-256" },
        "digest": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
      },
      "additionalProperties": false
    }
  },
  "$defs": {
    "identity": {
      "type": "string",
      "pattern": "^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$"
    },
    "immutable_version": {
      "type": "string",
      "pattern": "^(v?[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?)$"
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

For v2, `content_commitment.digest` is SHA-256 over the following unambiguous
preimage, with unsigned 64-bit big-endian byte lengths:

1. UTF-8 bytes of `quoin-proof-attestation-content/v2` followed by one NUL byte;
2. byte length of the canonical metadata JSON, then the canonical metadata JSON;
3. byte length of the retained output, then the exact retained output bytes.

Canonical metadata is the RFC 8785 JSON Canonicalization Scheme encoding of the
attestation object with only the top-level `content_commitment` member removed.
Duplicate JSON member names are refused before canonicalization. No whitespace,
newline, or output-byte normalization is performed. The digest is lowercase
hexadecimal prefixed by `sha256:`. Thus changing any attestation metadata or any
retained output byte changes the commitment; the digest field does not commit to
itself.

The commitment is a computed identity digest binding the retained output to its
attestation. It detects an output or metadata change made without recomputing
it, including an equal-length substitution. It is not a signature and not
independent retention: someone who can rewrite both the attestation and the
output can recompute it, and that protection is not provided here.

## Intake

The intake operation SHALL receive the attestation's exact UTF-8 JSON bytes plus
the exact retained output bytes. It SHALL use FR-063's duplicate-preserving raw
JSON boundary and SHALL check the schema, the retained output's byte length
against `retained_output.size_bytes`, and the v2 content commitment against the
canonical metadata and output bytes. `attestation_id` is assigned by
the caller (a UUID or caller-chosen identity), never derived from content.

It SHALL stage `attestation.json` and the output bytes together in a new
temporary directory, durably close both files, and atomically rename that
complete directory to `attestations/<attestation_id>`. A crash or error before
the rename SHALL leave no visible attestation or output; recovery SHALL discard
incomplete temporary directories. Repeating identical intake SHALL be
byte-idempotent (the stored bytes equal the supplied bytes, compared byte for
byte). An existing `attestation_id` with different bytes SHALL produce a
collision error and preserve the existing directory and both artifacts.

`unavailable` and `not_computed` attestations SHALL retain producer diagnostics
as their output bytes. Completely missing evidence has no attestation and is
represented by FR-065 as `missing_attestation`. No result, including `passed`,
discharges a proof until FR-065 checks all bindings and FR-032 findings.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-064-CON-1 | Intake SHALL execute no proof command, tool, Git operation, workflow, or network request. | Architecture | Inspection |
| FR-064-CON-2 | Intake SHALL neither infer a candidate revision, proof id, command, tool/configuration field, result, environment, nor output length. | Integrity | Test |
| FR-064-CON-3 | An attestation SHALL express no auditor verdict, approval, identity, authorization, or non-repudiation claim. | Responsibility | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-064-AC-1 | New intake requires schema v2 with the attestation identity, reviewed record id and revision, candidate revision, proof id, exact argv/cwd, immutable tool identity/version/configuration reference, environment, timestamp, result, retained-output media type and size, and SHA-256 content commitment; undeclared fields are refused. | Test |
| FR-064-AC-2 | Passed, failed, unavailable, and not-computed results remain four distinct producer states and none is converted into a verification outcome during intake. | Test |
| FR-064-AC-3 | The retained bytes have the declared size and match the v2 content commitment; an equal-length byte substitution, metadata substitution, absent or extra bytes is refused and leaves neither a new attestation nor output artifact. | Property |
| FR-064-AC-4 | The v2 commitment is byte-reproducible for equivalent metadata under RFC 8785 canonicalization, changes when any committed metadata or output byte changes, rejects duplicate JSON names, and uses the specified domain separator and length framing. | Test |
| FR-064-AC-5 | Each missing record, candidate, proof, argv/cwd, tool, configuration, environment, timestamp, result, or output field is refused independently rather than inferred. | Property |
| FR-064-AC-6 | Repeated identical intake preserves byte identity; injected failures before the directory rename expose neither artifact and are recoverable; a same-id/different-content collision preserves the first attestation and output. | Integration |
| FR-064-AC-7 | An unavailable or not-computed producer result retains its diagnostic output, while entirely missing evidence creates no synthetic attestation. | Test |
| FR-064-AC-8 | Existing FR-030 run evidence remains readable and unchanged; proof attestations occupy a distinct versioned store family. | Integration |
| FR-064-AC-9 | Static boundaries and golden terminology prove intake runs nothing and reports only producer facts, never an audit, approval, identity, authorization, or non-repudiation conclusion (CON-1, CON-3). | Analysis |
| FR-064-AC-10 | Each of Engineering Assurance's four claim strengths (FR-022) is written and reads back as itself; an attestation with no `strength` is refused by name rather than defaulted, and a `strength` outside the four wire names is refused as malformed. | Test |

## Dependencies

- **Upstream**: [FR-063](./FR-063-change-assurance-record-integrity.md)
  defines the reviewed proof and the shared raw JSON boundary; [FR-030](./FR-030-evidence-store.md)
  defines the retained-evidence boundary.
- **Downstream**: [FR-065](./FR-065-change-assurance-verification.md)
  selects and audits attestations into a verification receipt.
