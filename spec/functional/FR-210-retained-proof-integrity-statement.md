---
id: FR-210
title: "Retain signed proof integrity statements in independent storage"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quoin/US-017"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-064"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-080"
    type: "requires"
---

# FR-210: Retain signed proof integrity statements in independent storage

## Description

For each selected proof, Quoin SHALL create one canonical
`proof-integrity-statement/v1` that binds the reviewed record and candidate to
the exact retained attestation and output content.

Quoin SHALL obtain a purpose-bound signed tag for the exact statement digest
from the existing signing workflow. Quoin SHALL use Engineering Assurance
[FR-080](ix://agent-ix/engineering-assurance/FR-080) to verify that statement
digest against the exact immutable signed tag object and configured trust.

Quoin SHALL retain the exact statement, signed tag object, attestation bytes,
and output bytes in a retention service selected and independently controlled
by the organization. The service and its live behavior are deployment
prerequisites; a Quoin-local copy or Git history alone is insufficient. This
requirement establishes content integrity binding only. It does not establish
proof correctness, human approval, role authorization, or universal auditor
acceptance.

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| schema_version | Integer | 1..1 | min: 1, max: 1 |
| purpose | String | 1..1 | enumValues: proof-integrity-statement/v1 |
| record_id | String | 1..1 | minLength: 1, maxLength: 256 |
| record_revision | Integer | 1..1 | min: 1 |
| candidate_revision | String | 1..1 | minLength: 1 |
| proof_id | String | 1..1 | minLength: 1, maxLength: 256 |
| attestation_id | String | 1..1 | minLength: 1, maxLength: 256 |
| attestation_sha256 | String | 1..1 | pattern: /^sha256:[a-f0-9]{64}$/ |
| output_sha256 | String | 1..1 | pattern: /^sha256:[a-f0-9]{64}$/ |
| output_size_bytes | Integer | 1..1 | min: 0 |
| output_media_type | String | 1..1 | minLength: 1 |
| statement_digest | String | 1..1 | pattern: /^sha256:[a-f0-9]{64}$/ |

## Schema

`statement_digest` is SHA-256 over the following preimage, using an unsigned
64-bit big-endian byte length:

1. UTF-8 bytes of `quoin-proof-integrity-statement/v1` followed by one NUL byte;
2. the byte length of the RFC 8785 canonical JSON statement with
   `statement_digest` removed, followed by those exact canonical bytes.

Duplicate JSON member names are refused before canonicalization. The signed tag
checkpoint SHALL state purpose `proof-integrity-statement/v1` and the exact
`statement_digest`; it SHALL bind the same record/revision/candidate/proof and
attestation identifiers.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/proof-integrity-statement-v1.schema.json",
  "title": "ProofIntegrityStatementV1",
  "type": "object",
  "required": [
    "schema_version",
    "purpose",
    "record_id",
    "record_revision",
    "candidate_revision",
    "proof_id",
    "attestation_id",
    "attestation_sha256",
    "output_sha256",
    "output_size_bytes",
    "output_media_type",
    "statement_digest"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "purpose": { "const": "proof-integrity-statement/v1" },
    "record_id": { "$ref": "#/$defs/identity" },
    "record_revision": { "type": "integer", "minimum": 1 },
    "candidate_revision": { "type": "string", "minLength": 1 },
    "proof_id": { "$ref": "#/$defs/identity" },
    "attestation_id": { "$ref": "#/$defs/identity" },
    "attestation_sha256": { "$ref": "#/$defs/sha256_digest" },
    "output_sha256": { "$ref": "#/$defs/sha256_digest" },
    "output_size_bytes": { "type": "integer", "minimum": 0 },
    "output_media_type": { "type": "string", "minLength": 1 },
    "statement_digest": { "$ref": "#/$defs/sha256_digest" }
  },
  "additionalProperties": false,
  "$defs": {
    "identity": {
      "type": "string",
      "pattern": "^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$"
    },
    "sha256_digest": {
      "type": "string",
      "pattern": "^sha256:[a-f0-9]{64}$"
    }
  }
}
```

## Inputs

- The exact accepted `ProofAttestation` bytes and retained output bytes from
  [FR-064](./FR-064-proof-attestation.md).
- The configured signed-tag request for purpose, statement digest, exact proof
  scope, signer/trust policy, and tag reference. No caller-supplied expected
  tag object id is trusted as an anchor.
- The organization's selected retention service and its non-secret retention
  configuration.
- The checkpoint verification result from
  [Engineering Assurance FR-080](ix://agent-ix/engineering-assurance/FR-080).

## Outputs

- The canonical `proof-integrity-statement/v1`, its statement digest, and exact
  signed tag object id returned by FR-080 for the object it verified.
- The verified status or refusal/unavailable cause from FR-080.
- The retention service's returned object identifiers and operation outcomes.
  Quoin SHALL NOT emit credentials.

## Behavior

- Quoin SHALL populate record/revision/candidate/proof/attestation fields from
  the selected approved record and accepted attestation.
- Quoin SHALL compute `attestation_sha256` from the exact retained attestation
  bytes and `output_sha256` from the exact retained output bytes.
- Quoin SHALL compute `output_size_bytes` from those exact output bytes.
- Quoin SHALL copy `output_media_type` from the accepted attestation.
- Quoin SHALL serialize the statement as RFC 8785 canonical JSON before
  calculating `statement_digest`.
- When Quoin evaluates a proof for validity, it SHALL submit the exact
  statement digest, purpose, and proof scope to Engineering Assurance FR-080.
- FR-080 SHALL parse checkpoint fields and verify the signature from the same
  immutable annotated tag object fetched for one verification operation.
- FR-080 SHALL return the full object id of that exact annotated tag object
  with its parsed checkpoint and signature result.
- FR-080 SHALL return a non-verified result if its parsed fields and signature
  verdict do not refer to the same immutable tag object id.
- Quoin SHALL accept the checkpoint only when FR-080 verifies the expected
  statement digest and scope from that same object and returns its exact object
  id with the result.
- Quoin SHALL report the returned object id as the checkpoint object id; in the
  absence of a separate trusted anchor, Quoin SHALL NOT claim that it pins this
  id to a prior checkpoint.
- A second tag signed by the same trusted identity over the same purpose,
  statement digest, and scope SHALL be a distinct valid checkpoint when FR-080
  verifies that tag's own immutable object. FR-080 SHALL return that object's
  distinct object id.
- Quoin SHALL retain the exact statement, signed tag object, attestation bytes,
  and output bytes in the organization's selected retention service.
- Quoin SHALL compare bytes read from the service with the exact expected
  statement, tag, attestation, and output bytes before returning a valid proof.
- The selected retention service SHALL reject update and deletion attempts for
  each retained object under its configured policy. Quoin SHALL not claim that
  this behavior holds without repeatable verification against the selected real
  provider.
- Missing provider/configuration, unavailable service or verifier, missing
  object, or missing signature SHALL yield `incomplete` and never qualify a
  proof. A changed object, content/digest mismatch, wrong purpose/scope, or
  untrusted/bad signature SHALL yield `invalid` and never qualify a proof.
- Quoin SHALL keep statement integrity separate from human approval, signer
  role authorization, and proof-result adjudication.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-210-CON-1 | Quoin SHALL NOT treat its local store or Git history as the independent retention service for the statement and source objects. | Architecture | Inspection |
| FR-210-CON-2 | Engineering Assurance SHALL verify the statement digest and exact signed tag object using explicit trust configuration. | Responsibility | Integration |
| FR-210-CON-3 | The `proof-integrity-statement/v1` SHALL bind only one proof and its exact attestation/output content. | Integrity | Test |
| FR-210-CON-4 | Quoin SHALL NOT interpret statement verification as human approval, role authorization, or proof correctness. | Responsibility | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-210-AC-1 | Each canonical `proof-integrity-statement/v1` binds exactly one record ID/revision, candidate revision, proof ID, attestation ID, exact attestation-byte SHA-256, exact output-byte SHA-256, output size, and media type; undeclared fields and duplicate JSON names are rejected. | Test |
| FR-210-AC-2 | The statement digest is reproducible; changing any bound field or exact source byte changes the statement digest, and a signed tag for another purpose, statement digest, scope, or untrusted signer never verifies. | Test |
| FR-210-AC-3 | The exact statement, signed tag object, attestation bytes, and output bytes are retained and read back byte-for-byte from the selected service; unavailable or missing service/object evidence is incomplete and never qualifies. | Integration |
| FR-210-AC-4 | A repeatable integration test against the selected real provider closes and reopens the service client, copies the retained objects, attempts update and deletion of each object, and proves that exact bytes remain retrievable and the provider refuses both operations under the selected policy. The test evidence identifies provider, non-secret configuration, run/object IDs, digests, and results without credentials. | Integration |
| FR-210-AC-5 | Joint replacement of attestation metadata and output bytes, statement substitution, changed-length or equal-length content mutation, replay across proof/record/revision, missing or bad signature, and wrong signer, trust, purpose, digest, or scope produce incomplete or invalid evidence, never a valid proof. | Integration |
| FR-210-AC-6 | A deliberately removed statement-digest comparison, or a result that parses checkpoint fields from one object and verifies a signature from another, makes its corresponding negative-control test fail. | Test |
| FR-210-AC-7 | A successful checkpoint does not produce human approval, signer role authorization, or proof correctness, and the receipt preserves the statement digest, exact tag object id, and checkpoint status for the selected proof. | Test |
| FR-210-AC-8 | FR-080 parses and verifies one fetched immutable tag object atomically, returns that object's full id, and refuses a parsed-versus-verified object mismatch. A second trusted signature over the same purpose, statement digest, and scope may verify as a distinct checkpoint with its distinct returned object id; without a separate trusted anchor, Quoin claims no prior-object-id pinning. | Integration |

## Dependencies

- **Upstream**: [FR-064](./FR-064-proof-attestation.md) defines exact
  attestation and output bytes; [Engineering Assurance FR-080](ix://agent-ix/engineering-assurance/FR-080)
  verifies the signed statement digest against the exact tag object.
- **Downstream**: [FR-065](./FR-065-change-assurance-verification.md) uses
  this statement and retained content when determining whether selected proof
  evidence may qualify.
