---
id: FR-065
title: "Read-only change assurance verification receipt"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quoin/US-017"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-063"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-064"
    type: "requires"
  - target: "ix://agent-ix/ix-flow/FR-013"
    type: "requires"
  - target: "ix://agent-ix/ix-flow/FR-018"
    type: "requires"
---

# FR-065: Read-only change assurance verification receipt

## Description

When change assurance is verified, Quoin SHALL join one sealed
`ChangeAssuranceRecord`, its retained parent chain, ix-flow decision history,
one explicitly selected `ProofAttestation` per proof, the retained output bytes,
and the FR-032 audit report into one reproducible `VerificationReceipt`.

The verifier SHALL run no producer or proof command. Its three outcomes mean:

- `valid`: every required input is present, integrity-valid, exactly bound,
  approved, complete, and healthy;
- `invalid`: at least one retained input proves a contradiction, mismatch,
  failed proof, unhealthy evidence, rejected decision, or integrity failure; and
- `incomplete`: no invalid fact is present, but a required input, evaluation,
  approval, complete impact premise, resolved unknown, or available result is
  absent.

`invalid` SHALL dominate `incomplete`.

When no invalid check exists, any incomplete check SHALL dominate `valid`.

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/verification-receipt-v1.schema.json",
  "title": "VerificationReceiptV1",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "digest",
    "record_digest",
    "candidate_revision",
    "decision_event",
    "parent_digests",
    "checks",
    "proofs",
    "unknowns",
    "outcome",
    "reasons"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "record_type": { "const": "verification_receipt" },
    "digest": { "$ref": "#/$defs/digest" },
    "record_digest": { "$ref": "#/$defs/digest" },
    "candidate_revision": { "type": "string", "minLength": 1 },
    "decision_event": {
      "anyOf": [{ "$ref": "#/$defs/decision_event" }, { "type": "null" }]
    },
    "parent_digests": {
      "type": "array",
      "items": { "$ref": "#/$defs/digest" }
    },
    "checks": {
      "type": "object",
      "required": ["record", "lineage", "review", "impact"],
      "properties": {
        "record": { "$ref": "#/$defs/check" },
        "lineage": { "$ref": "#/$defs/check" },
        "review": { "$ref": "#/$defs/check" },
        "impact": { "$ref": "#/$defs/check" }
      },
      "additionalProperties": false
    },
    "proofs": {
      "type": "array",
      "items": { "$ref": "#/$defs/proof_receipt" }
    },
    "unknowns": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "disposition"],
        "properties": {
          "id": { "$ref": "#/$defs/identity" },
          "disposition": {
            "type": "string",
            "enum": ["open", "accepted", "deferred", "resolved"]
          }
        },
        "additionalProperties": false
      }
    },
    "outcome": { "$ref": "#/$defs/outcome" },
    "reasons": {
      "type": "array",
      "items": { "$ref": "#/$defs/reason" }
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
    "outcome": {
      "type": "string",
      "enum": ["valid", "invalid", "incomplete"]
    },
    "reason": {
      "type": "string",
      "enum": [
        "schema_invalid",
        "record_digest_mismatch",
        "parent_missing",
        "parent_invalid",
        "parent_mismatch",
        "revision_gap",
        "impact_incomplete",
        "impact_truncated",
        "unresolved_unknown",
        "decision_missing",
        "event_chain_missing",
        "event_chain_invalid",
        "decision_mismatch",
        "review_rejected",
        "review_revision_requested",
        "attestation_missing",
        "attestation_schema_invalid",
        "attestation_digest_mismatch",
        "output_missing",
        "output_digest_mismatch",
        "record_binding_mismatch",
        "candidate_revision_mismatch",
        "proof_id_mismatch",
        "command_mismatch",
        "tool_identity_mismatch",
        "configuration_mismatch",
        "result_failed",
        "result_unavailable",
        "result_not_computed",
        "evidence_stale",
        "evidence_suspect",
        "evidence_vacuous",
        "evidence_unrelated",
        "audit_finding",
        "audit_not_evaluated"
      ]
    },
    "check": {
      "type": "object",
      "required": ["outcome", "reasons"],
      "properties": {
        "outcome": { "$ref": "#/$defs/outcome" },
        "reasons": {
          "type": "array",
          "items": { "$ref": "#/$defs/reason" }
        }
      },
      "additionalProperties": false
    },
    "decision_event": {
      "type": "object",
      "required": ["run_id", "event_id", "event_hash", "chain_tail_hash", "recorded_actor", "decision"],
      "properties": {
        "run_id": { "$ref": "#/$defs/identity" },
        "event_id": { "$ref": "#/$defs/identity" },
        "event_hash": { "$ref": "#/$defs/digest" },
        "chain_tail_hash": { "$ref": "#/$defs/digest" },
        "recorded_actor": { "type": "string", "minLength": 1 },
        "decision": {
          "type": "string",
          "enum": ["approved", "rejected", "revise"]
        }
      },
      "additionalProperties": false
    },
    "audit_finding": {
      "type": "object",
      "required": ["obligation_id", "kind"],
      "properties": {
        "obligation_id": { "$ref": "#/$defs/identity" },
        "kind": { "type": "string", "minLength": 1 }
      },
      "additionalProperties": false
    },
    "proof_receipt": {
      "type": "object",
      "required": [
        "proof_id",
        "obligation_ids",
        "attestation_digest",
        "retained_output_digest",
        "audit_report_digest",
        "audit_findings",
        "outcome",
        "reasons"
      ],
      "properties": {
        "proof_id": { "$ref": "#/$defs/identity" },
        "obligation_ids": {
          "type": "array",
          "minItems": 1,
          "items": { "$ref": "#/$defs/identity" }
        },
        "attestation_digest": {
          "anyOf": [{ "$ref": "#/$defs/digest" }, { "type": "null" }]
        },
        "retained_output_digest": {
          "anyOf": [{ "$ref": "#/$defs/digest" }, { "type": "null" }]
        },
        "audit_report_digest": {
          "anyOf": [{ "$ref": "#/$defs/digest" }, { "type": "null" }]
        },
        "audit_findings": {
          "type": "array",
          "items": { "$ref": "#/$defs/audit_finding" }
        },
        "outcome": { "$ref": "#/$defs/outcome" },
        "reasons": {
          "type": "array",
          "items": { "$ref": "#/$defs/reason" }
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## Selection and checks

The verifier input SHALL carry an explicit array of selection entries, each
containing exactly `proof_id` and `attestation_digest`; no selection is inferred
from the store. Quoin SHALL group that raw list before constructing a map so a
repeated proof id remains observable. It SHALL emit exactly one proof receipt
for every proof in the reviewed record, ordered by proof id. A missing selection
is `incomplete`; more than one selection for a proof is `invalid` with
`attestation_schema_invalid`; and a selected attestation bound to another
record, candidate, or proof is `invalid`. A selection for a proof absent from
the record is invalid with `proof_id_mismatch`. Unselected stored attestations
have no effect.

For each selected attestation Quoin SHALL:

1. verify the attestation and exact retained-output bytes;
2. require equal record digest, candidate revision, proof id, command argv/cwd,
   tool identity, and configuration digest;
3. require producer result `passed`; and
4. join the proof's declared obligation ids to the retained FR-032 audit report.

Each FR-032 stale, suspect, or vacuous finding SHALL map to its named invalid
reason. A failed result and every other FR-032 defect finding SHALL be invalid.
An unavailable or not-computed result, missing evidence, or FR-032
`not-evaluated` state SHALL be incomplete. The receipt SHALL retain every exact
FR-032 finding kind and obligation id rather than collapse it into the mapped
reason.

The review check SHALL verify the complete ix-flow event chain under ix-flow
FR-013 and require exactly one matching decision event. `approved` permits a
valid review check; `rejected` and `revise` are invalid. Zero matching events or
missing history is incomplete; more than one matching event is invalid with
`decision_mismatch`; and a broken chain or mismatched event is invalid.

An incomplete or truncated impact snapshot or any unknown whose disposition is
not `resolved` SHALL make the receipt incomplete. It SHALL remain visible so an
advisory workflow may proceed without misreporting complete verification.

## Determinism and integrity

The receipt SHALL order parent digests from revision 1 through the immediate
parent, proofs by proof id, findings by obligation id then kind, unknowns by id,
and reasons lexicographically by code.

Quoin SHALL compute `digest` with FR-063's RFC 8785/BLAKE3 procedure after
removing only the top-level `digest` member.

Identical retained inputs and selections SHALL produce byte-identical receipt
JSON.

The receipt's `recorded_actor` is copied from the ix-flow event. Neither it, the
event hash, nor any BLAKE3 digest establishes identity, authorization,
authenticity, signature, or non-repudiation.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-065-CON-1 | Verification SHALL execute no proof, suite, tool, ix-flow command, Git command, network request, or evidence write. | Architecture | Inspection |
| FR-065-CON-2 | Verification SHALL consume FR-032 findings without replacing or suppressing their auditor verdict. | Responsibility | Test |
| FR-065-CON-3 | A receipt SHALL make no identity, authorization, authenticity, signature, or non-repudiation claim. | Responsibility | Inspection |
| FR-065-CON-4 | Existing evidence-audit and assurance reports SHALL remain readable and unchanged when change assurance is not selected. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-065-AC-1 | Receipt schema v1 records the exact record/candidate, decision event and chain tail, parent chain, record/lineage/review/impact checks, every proof and auditor finding, unknowns, overall outcome, reasons, and receipt digest; undeclared fields are refused. | Test (TC-1281) |
| FR-065-AC-2 | Valid, invalid, and incomplete remain distinct; any invalid check dominates incomplete, any incomplete check dominates valid, and an empty reason set is permitted only for valid. | Property (TC-1282) |
| FR-065-AC-3 | Exactly one proof receipt is emitted per reviewed proof in id order; missing selection is incomplete, duplicate or unknown-proof selection is invalid, and unselected stored attestations do not affect the result. | Property (TC-1283) |
| FR-065-AC-4 | Changing record digest, candidate revision, proof id, argv/cwd, tool identity, or configuration independently makes the selected attestation invalid and names the mismatched premise. | Property (TC-1284) |
| FR-065-AC-5 | Passed evidence can discharge only when its exact retained bytes verify and all owning obligations are healthy; failed, stale, suspect, vacuous, unrelated, output-mismatched, or other defect evidence is invalid. | Property (TC-1285) |
| FR-065-AC-6 | A missing attestation or output, unavailable or not-computed result, or audit not-evaluated state is incomplete and never valid or numeric zero. | Test (TC-1286) |
| FR-065-AC-7 | Exactly one intact matching human `approved` event permits review validity; missing history/event is incomplete; duplicate matching decisions, broken chain, mismatched event, rejection, or revise request are invalid. | Integration (TC-1287) |
| FR-065-AC-8 | Incomplete/truncated impact evidence and every non-resolved unknown remain named and force incomplete verification while preserving an advisory workflow's ability to continue. | Test (TC-1288) |
| FR-065-AC-9 | The receipt retains each FR-032 finding kind and obligation id byte-for-byte beside its mapped reason without changing the source auditor result (CON-2). | Integration (TC-1289) |
| FR-065-AC-10 | Input and selection permutations emit byte-identical canonical receipts whose digest matches the pinned RFC 8785/BLAKE3 procedure. | Property (TC-1290) |
| FR-065-AC-11 | Existing audit/assurance goldens stay byte-identical without change-assurance selection, and static boundaries prove verification runs and writes nothing (CON-1, CON-4). | Integration (TC-1291) |
| FR-065-AC-12 | Golden terminology describes actor labels and hashes only as recorded attribution and content integrity, never identity, authority, authenticity, signature, or non-repudiation (CON-3). | Analysis (TC-1292) |

## Dependencies

- **Upstream**: [FR-032](./FR-032-evidence-auditor.md),
  [FR-063](./FR-063-change-assurance-record-integrity.md),
  [FR-064](./FR-064-proof-attestation.md), and ix-flow FR-013/018.
- **Downstream**: advisory pilots and explicitly promoted assurance profiles may
  consume the outcome; this requirement defines no merge gate or promotion
  policy.
