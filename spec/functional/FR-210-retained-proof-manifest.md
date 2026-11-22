---
id: FR-210
title: "Retain proof commitments in an independently protected manifest"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quoin/US-017"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-064"
    type: "requires"
---

# FR-210: Retain proof commitments in an independently protected manifest

## Description

Quoin SHALL retain one canonical manifest entry for each selected
`ProofAttestation` and its exact output bytes.

Quoin SHALL store the entry chain in a retention service selected by the
organization.

The organization SHALL administer the selected retention service independently
of the Quoin evidence store and configure append-only/WORM retention or a
documented equivalent that it assesses.

Quoin SHALL NOT treat a local file, Git history, or hash chain alone as the
independent retention service.

When Quoin verifies a selected proof, it SHALL verify the signed manifest
checkpoint through Engineering Assurance's distinct checkpoint verifier.

The signed checkpoint SHALL bind the exact manifest head, record id and
revision, candidate revision, and complete selected proof population.

Quoin SHALL compare the verified checkpoint values with the approved change
definition and retained manifest entries. The checkpoint authenticates an
integrity commitment only; it is not human approval or proof qualification.

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| schema_version | Integer | 1..1 | min: 1, max: 1 |
| record_type | String | 1..1 | enumValues: proof_manifest_entry |
| manifest_id | String | 1..1 | minLength: 1, maxLength: 256 |
| sequence | Integer | 1..1 | min: 1 |
| previous_entry_digest | String | 1..1 | pattern: /^sha256:[a-f0-9]{64}$/; sequence 1 uses the all-zero digest |
| record_id | String | 1..1 | minLength: 1, maxLength: 256 |
| record_revision | Integer | 1..1 | min: 1 |
| candidate_revision | String | 1..1 | minLength: 1 |
| proof_id | String | 1..1 | minLength: 1, maxLength: 256 |
| attestation_id | String | 1..1 | minLength: 1, maxLength: 256 |
| attestation_sha256 | String | 1..1 | pattern: /^sha256:[a-f0-9]{64}$/; exact attestation bytes |
| output_sha256 | String | 1..1 | pattern: /^sha256:[a-f0-9]{64}$/; exact output bytes |
| output_media_type | String | 1..1 | minLength: 1 |
| output_size_bytes | Integer | 1..1 | min: 0 |
| entry_digest | String | 1..1 | pattern: /^sha256:[a-f0-9]{64}$/ |

## Schema

`entry_digest` is SHA-256 over the following preimage, using unsigned 64-bit
big-endian byte lengths:

1. UTF-8 bytes of `quoin-proof-manifest-entry/v1` followed by one NUL byte;
2. the byte length of the RFC 8785 canonical JSON entry with `entry_digest`
   removed, followed by those exact canonical bytes.

`previous_entry_digest` is `sha256:` followed by 64 zeroes for sequence 1.
Sequence N greater than 1 carries the `entry_digest` of sequence N-1 in the
same manifest.
Duplicate JSON member names SHALL be refused by Quoin before canonicalization.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/proof-manifest-entry-v1.schema.json",
  "title": "ProofManifestEntryV1",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "manifest_id",
    "sequence",
    "previous_entry_digest",
    "record_id",
    "record_revision",
    "candidate_revision",
    "proof_id",
    "attestation_id",
    "attestation_sha256",
    "output_sha256",
    "output_media_type",
    "output_size_bytes",
    "entry_digest"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "record_type": { "const": "proof_manifest_entry" },
    "manifest_id": { "type": "string", "minLength": 1, "maxLength": 256 },
    "sequence": { "type": "integer", "minimum": 1 },
    "previous_entry_digest": { "$ref": "#/$defs/sha256_digest" },
    "record_id": { "type": "string", "minLength": 1, "maxLength": 256 },
    "record_revision": { "type": "integer", "minimum": 1 },
    "candidate_revision": { "type": "string", "minLength": 1 },
    "proof_id": { "type": "string", "minLength": 1, "maxLength": 256 },
    "attestation_id": { "type": "string", "minLength": 1, "maxLength": 256 },
    "attestation_sha256": { "$ref": "#/$defs/sha256_digest" },
    "output_sha256": { "$ref": "#/$defs/sha256_digest" },
    "output_media_type": { "type": "string", "minLength": 1 },
    "output_size_bytes": { "type": "integer", "minimum": 0 },
    "entry_digest": { "$ref": "#/$defs/sha256_digest" }
  },
  "allOf": [
    {
      "if": { "properties": { "sequence": { "const": 1 } } },
      "then": {
        "properties": {
          "previous_entry_digest": {
            "const": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
          }
        }
      }
    },
    {
      "if": { "properties": { "sequence": { "minimum": 2 } } },
      "then": {
        "properties": {
          "previous_entry_digest": { "$ref": "#/$defs/sha256_digest" }
        }
      }
    }
  ],
  "additionalProperties": false,
  "$defs": {
    "sha256_digest": {
      "type": "string",
      "pattern": "^sha256:[a-f0-9]{64}$"
    }
  }
}
```

## Inputs

- The exact retained attestation bytes and exact retained output bytes accepted
  by [FR-064](./FR-064-proof-attestation.md).
- The selected proof population derived from the approved change definition.
- The signed checkpoint result returned by Engineering Assurance's checkpoint
  verifier, including the exact tag object id, purpose, manifest-head digest,
  record and candidate revisions, proof population, signer, and trust result.
- The organization's retention-service selection and its retention policy
  configuration, without credentials.

## Outputs

- The canonical manifest entry and the retention service's append result.
- A verification state of `valid`, `invalid`, or `incomplete`, with provider
  identity, configuration reference, run/object ids, digests, and refusal or
  unavailable causes. Credentials SHALL NOT be emitted.

## Behavior

- Quoin SHALL derive each entry's record, revision, candidate, proof, and
  attestation identity from the accepted `ProofAttestation`.
- Quoin SHALL compute the attestation digest over the exact stored attestation
  bytes.
- Quoin SHALL compute the output digest over the exact retained output bytes.
- Quoin SHALL submit entries to the selected retention service in contiguous
  sequence order.
- Quoin SHALL serialize each entry as RFC 8785 canonical JSON before append.
- When Quoin reads an entry back, it SHALL compare the exact stored entry bytes
  with the canonical bytes it derives for that entry.
- The selected retention service SHALL preserve committed entries against
  update and deletion under its configured append-only/WORM policy or assessed
  equivalent.
- The selected retention service SHALL treat an exact duplicate append as
  idempotent.
- Quoin SHALL reject a conflicting entry at an occupied sequence, sequence gap,
  fork, cross-scope entry, or rollback without producing a qualifying result.
- Quoin SHALL append no entry when no organization-selected retention service
  is configured. A missing, unavailable, or unverified retention service SHALL
  produce `incomplete`, never a positive proof qualification.
- When Quoin evaluates a selected proof for validity, it SHALL obtain the signed
  checkpoint verification result from Engineering Assurance.
- Quoin SHALL compare the verified tag object id, purpose, manifest-head
  digest, record id and revision, candidate revision, and proof population with
  the approved change definition and the complete contiguous manifest chain.
- Quoin SHALL require the entry proof-id set to equal the complete selected
  proof-id set, with no duplicate or extra proof ids.
- A changed or missing entry under a present checkpoint, mismatched digest,
  wrong record/candidate scope, chain fork, sequence gap, or population
  mismatch SHALL be `invalid`. A missing checkpoint, missing legacy manifest,
  or unavailable verifier/service SHALL be `incomplete`. Neither state SHALL
  qualify a proof.
- A manifest without a verified signed checkpoint SHALL not be treated as an
  independent integrity anchor. Joint substitution of attestation metadata and
  output bytes SHALL change the entry chain head and fail checkpoint comparison.
- Quoin SHALL retain the provider identity, non-secret policy/configuration
  reference, append/read run ids, object digests, and observed outcomes as
  verification evidence. It SHALL make no universal WORM or auditor-acceptance
  claim beyond the selected provider's evidenced behavior and recorded
  organizational assessment.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-210-CON-1 | The organization SHALL administer the selected retention service independently of the Quoin evidence store. | Architecture | Inspection |
| FR-210-CON-2 | Quoin SHALL NOT use local atomic writes, Git history, or a local hash chain as the sole retention protection. | Architecture | Inspection |
| FR-210-CON-3 | Engineering Assurance SHALL verify the checkpoint against the exact signed tag object and explicit trust configuration. | Responsibility | Integration |
| FR-210-CON-4 | Quoin SHALL NOT interpret human approval as checkpoint approval. | Responsibility | Inspection |
| FR-210-CON-5 | Manifest retention SHALL NOT execute a proof. | Responsibility | Inspection |
| FR-210-CON-6 | Manifest retention SHALL NOT convert the producer's result into an auditor verdict. | Responsibility | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-210-AC-1 | Each manifest entry binds the exact record id/revision, candidate revision, proof id, attestation id, exact attestation-byte SHA-256, exact output-byte SHA-256, output size/media type, sequence, predecessor digest, and canonical entry digest; duplicate JSON keys and undeclared fields are refused. | Test |
| FR-210-AC-2 | Canonical entry bytes and digest are reproducible; changing any bound metadata or bytes changes the digest, and a chain with a gap, fork, duplicate sequence conflict, or rollback is refused or invalid. | Property |
| FR-210-AC-3 | Repeating an identical append is idempotent; a conflicting append cannot replace an existing entry or advance a partial chain. | Integration |
| FR-210-AC-4 | With a selected real retention service, committed entries remain retrievable and unchanged after process close/reopen and an independent copy; the result records provider identity, non-secret retention configuration, run ids, object digests, and outcomes. | Demonstration |
| FR-210-AC-5 | Missing provider/configuration, unavailable service/verifier, missing checkpoint, or legacy evidence without a checkpoint is incomplete and never qualifies; contradictory digest, changed entry, wrong checkpoint purpose/tag/head, rollback, or wrong scope is invalid and never qualifies. | Integration |
| FR-210-AC-6 | The verified checkpoint head and complete proof population match the approved change definition and every manifest entry; missing, duplicate, extra, or cross-record/revision proof entries cannot produce a valid result. | Integration |
| FR-210-AC-7 | Metadata-only substitution and joint substitution of attestation metadata plus output bytes change the manifest head and fail against the signed checkpoint; deliberately removing the checkpoint comparison causes the negative-control test to fail. | Mutation |
| FR-210-AC-8 | A local hash chain, local atomic write, or Git history without an independently controlled selected retention service and verified signed checkpoint never satisfies this requirement. | Analysis |

## Dependencies

- **Upstream**: [FR-064](./FR-064-proof-attestation.md) defines the exact
  attestation and retained output bytes. Engineering Assurance owns the
  separate signed checkpoint verifier and exact tag-object binding.
- **Downstream**: [FR-065](./FR-065-change-assurance-verification.md) consumes
  the retention and checkpoint result when determining whether selected proof
  evidence may qualify.
