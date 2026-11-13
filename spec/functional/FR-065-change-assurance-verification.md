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

When change assurance is verified, Quoin SHALL join one
`ChangeAssuranceRecord` revision, its retained predecessor revisions, ix-flow decision history,
one explicitly selected `ProofAttestation` per proof, the retained output bytes,
and the FR-032 audit report into one reproducible `VerificationReceipt`.

The verifier SHALL run no producer or proof command. Its three outcomes mean:

- `valid`: every required input is present, exactly bound,
  approved, complete, and healthy;
- `invalid`: at least one retained input proves a contradiction, mismatch,
  failed proof, unhealthy evidence, or rejected decision; and
- `incomplete`: no invalid fact is present, but a required input, evaluation,
  approval, complete impact premise, resolved unknown, or available result is
  absent.

`invalid` SHALL dominate `incomplete`.

When no invalid check exists, any incomplete check SHALL dominate `valid`.

## Properties

| Field               | Type       | Multiplicity | Constraints                            |
| ------------------- | ---------- | ------------ | -------------------------------------- |
| schema_version      | Integer    | 1..1         |                                        |
| record_type         | String     | 1..1         | enumValues: verification_receipt       |
| record_id           | String     | 1..1         | minLength: 1, maxLength: 256           |
| record_revision     | Integer    | 1..1         | min: 1                                 |
| candidate_revision  | String     | 1..1         | minLength: 1                           |
| decision_event      | JsonObject | 1..1         |                                        |
| checks              | JsonObject | 1..1         |                                        |
| proofs              | JsonObject | 0..*         |                                        |
| unknowns            | JsonObject | 0..*         |                                        |
| outcome             | String     | 1..1         | enumValues: valid\|invalid\|incomplete |
| reasons             | JsonObject | 0..*         |                                        |
| governing_plan_id   | JsonObject | 1..1         |                                        |
| diff_paths_supplied | Boolean    | 1..1         |                                        |

The table declares the record's top-level fields and their cardinality. Which keys are required, which values may be null, and the cross-field rules are stated only by the schema below, which is the full contract.

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/verification-receipt-v2.schema.json",
  "title": "VerificationReceiptV2",
  "$comment": "governing_plan_id and diff_paths_supplied (PLAT-1015, FR-111) and claimed_approval are required on every receipt. Version 1 receipts are not read.",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "record_id",
    "record_revision",
    "candidate_revision",
    "decision_event",
    "checks",
    "proofs",
    "unknowns",
    "outcome",
    "reasons",
    "governing_plan_id",
    "diff_paths_supplied"
  ],
  "properties": {
    "schema_version": { "const": 2 },
    "record_type": { "const": "verification_receipt" },
    "record_id": { "$ref": "#/$defs/identity" },
    "record_revision": { "type": "integer", "minimum": 1 },
    "candidate_revision": { "type": "string", "minLength": 1 },
    "decision_event": {
      "anyOf": [{ "$ref": "#/$defs/decision_event" }, { "type": "null" }]
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
    },
    "governing_plan_id": {
      "anyOf": [{ "type": "string", "minLength": 1 }, { "type": "null" }]
    },
    "diff_paths_supplied": { "type": "boolean" }
  },
  "$defs": {
    "identity": {
      "type": "string",
      "pattern": "^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$"
    },
    "outcome": {
      "type": "string",
      "enum": ["valid", "invalid", "incomplete"]
    },
    "reason": {
      "type": "string",
      "enum": [
        "schema_invalid",
        "record_revision_mismatch",
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
        "approval_record_mismatch",
        "approval_signer_untrusted",
        "approval_signature_bad",
        "approval_unverified",
        "approval_meaning_missing",
        "approval_missing",
        "review_rejected",
        "review_revision_requested",
        "attestation_missing",
        "attestation_schema_invalid",
        "attestation_id_mismatch",
        "output_missing",
        "output_size_mismatch",
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
        "evidence_vacuous",
        "evidence_unrelated",
        "audit_finding",
        "audit_not_evaluated",
        "apparatus_touched",
        "negative_control_uncaught",
        "diff_missing"
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
      "required": [
        "run_id",
        "event_id",
        "sequence",
        "recorded_actor",
        "decision",
        "claimed_approval"
      ],
      "properties": {
        "run_id": { "$ref": "#/$defs/identity" },
        "event_id": { "$ref": "#/$defs/identity" },
        "sequence": { "type": "integer", "minimum": 1 },
        "recorded_actor": { "type": "string", "minLength": 1 },
        "decision": {
          "type": "string",
          "enum": ["approved", "rejected", "revise"]
        },
        "claimed_approval": {
          "anyOf": [{ "$ref": "#/$defs/claimed_approval" }, { "type": "null" }]
        }
      },
      "additionalProperties": false
    },
    "claimed_approval": {
      "type": "object",
      "required": [
        "protocol",
        "tag",
        "record_digest",
        "trust",
        "approver",
        "meaning",
        "signature_kind"
      ],
      "properties": {
        "protocol": {
          "const": "engineering-assurance.approval-verify-result/v1"
        },
        "tag": { "type": "string", "minLength": 1 },
        "trust": {
          "type": "object",
          "required": ["allowed_signers"],
          "properties": {
            "allowed_signers": { "type": "string", "minLength": 1 },
            "gitsign": {
              "type": "object",
              "required": [
                "certificate_identity",
                "certificate_oidc_issuer",
                "principal"
              ],
              "properties": {
                "certificate_identity": { "type": "string", "minLength": 1 },
                "certificate_oidc_issuer": { "type": "string", "minLength": 1 },
                "principal": { "type": "string", "minLength": 1 }
              },
              "additionalProperties": false
            }
          },
          "additionalProperties": false
        },
        "record_digest": { "type": "string", "minLength": 1, "maxLength": 256 },
        "approver": { "type": "string", "minLength": 1 },
        "meaning": {
          "type": "array",
          "minItems": 1,
          "items": { "type": "string" }
        },
        "signature_kind": { "type": "string", "enum": ["ssh", "gitsign"] }
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
        "attestation_id",
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
        "attestation_id": {
          "anyOf": [{ "$ref": "#/$defs/identity" }, { "type": "null" }]
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
containing exactly `proof_id` and `attestation_id`; no selection is inferred
from the store. Quoin SHALL group that raw list before constructing a map so a
repeated proof id remains observable. It SHALL emit exactly one proof receipt
for every proof in the reviewed record, ordered by proof id. A missing selection
is `incomplete`; more than one selection for a proof is `invalid` with
`attestation_schema_invalid`; and a selected attestation bound to another
record, candidate, or proof is `invalid`. A selection for a proof absent from
the record is invalid with `proof_id_mismatch`. Unselected stored attestations
have no effect.

For each selected attestation Quoin SHALL:

1. read the attestation and its retained output and check the output's byte length equals `retained_output.size_bytes`;
2. require equal record id and revision, candidate revision, proof id, command argv/cwd,
   tool identity, and configuration reference;
3. require producer result `passed`; and
4. join the proof's declared obligation ids to the retained FR-032 audit report.

Each FR-032 stale or vacuous finding SHALL map to its named invalid
reason. A failed result and every other FR-032 defect finding SHALL be invalid.
An unavailable or not-computed result, missing evidence, or FR-032
`not-evaluated` state SHALL be incomplete. The receipt SHALL retain every exact
FR-032 finding kind and obligation id rather than collapse it into the mapped
reason.

The `apparatus_touched` (invalid), `diff_missing` (incomplete) and
`negative_control_uncaught` (incomplete) reasons are defined by FR-111, which
judges a retained diff against a linked measurement plan's protected apparatus.

The review check SHALL read the run's ix-flow decision events in `sequence` order
and require exactly one matching decision event. Sequences must be 1..n with no
gap or repeat; no event hash is read. `approved` permits a
valid review check; `rejected` and `revise` are invalid. Zero matching events or
missing history is incomplete; more than one matching event is invalid with
`decision_mismatch`; and a gap or repeat in the sequence, or a mismatched event, is invalid.

### Signed approval

The reviewed record's `review_workflow.signed_approval` (FR-063) governs this
section. `verified_approval` is the event writer's claim (FR-063): Quoin compares
it with the record and the policy and never verifies the signature. The receipt
reports it as `claimed_approval`, to be re-verified (FR-068) before anyone relies
on the approver.

When `signed_approval.required` is false and the matching `approved` event carries
no `verified_approval`, the review check is judged as above and `claimed_approval`
is `null`. When `required` is true and the event carries none, the review check is
`incomplete` with `approval_missing`. When the event carries one, the review check
SHALL additionally apply the first rule below that holds. The order follows
Engineering Assurance FR-041: signature status, then trust, then record binding,
then meaning.

1. status `signed_bad` is `invalid` with `approval_signature_bad`; status
   `untrusted_signer` is `invalid` with `approval_signer_untrusted`; status
   `unsigned` or `unverifiable` is `incomplete` with `approval_unverified`;
2. when the record's `allowed_signers_path` is non-null and differs from
   `trust.allowed_signers`, `invalid` with `approval_signer_untrusted`;
3. status `wrong_record`, or a `record_id` other than the record's, or a
   `record_digest` other than the digest of the reviewed revision's stored bytes
   (FR-063), is `invalid` with `approval_record_mismatch`;
4. status `meaning_missing` is `incomplete` with `approval_meaning_missing`;
5. status `approved` with all of the above satisfied leaves the review check as
   above and fills `claimed_approval` with `protocol`, `tag`, `record_digest`,
   `trust`, `approver`, `meaning` and `signature_kind` from the claim.

`claimed_approval` is `null` in every case except rule 5.

Duplicate decisions stay `invalid` with `decision_mismatch` under the exactly-one
rule: when two matching events carry approvals, none is selected. Quoin recomputes the
digest from the stored record and compares strings; it starts no Git, network or
identity-provider operation and does not verify any signature.

An incomplete or truncated impact snapshot or any unknown whose disposition is
not `resolved` SHALL make the receipt incomplete. It SHALL remain visible so an
advisory workflow may proceed without misreporting complete verification.

## Determinism

The receipt SHALL order proofs by proof id, findings by obligation id then kind,
unknowns by id, and reasons lexicographically by code. The receipt carries no
digest or id of its own.

Identical retained inputs and selections SHALL produce byte-identical receipt
JSON.

The receipt's `recorded_actor` is copied from the ix-flow event. It does not establish identity, authorization,
authenticity, signature, or non-repudiation. The receipt's `claimed_approval` member,
when non-null, is the event writer's claim, retained with what re-verification needs;
it is evidence that the named approver signed only after Engineering Assurance's
`approval-verify` is re-run on it with the same result.

The receipt schema is revised to version 2 by the `claimed_approval` member and the
six `approval_*` reasons; version 1 receipts are not read (prerelease, no
compatibility reader).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-065-CON-1 | Verification SHALL execute no proof, suite, tool, ix-flow command, Git command, network request, or evidence write. | Architecture | Inspection |
| FR-065-CON-2 | Verification SHALL consume FR-032 findings without replacing or suppressing their auditor verdict. | Responsibility | Test |
| FR-065-CON-3 | A receipt SHALL make no identity, authorization, authenticity, signature, or non-repudiation claim of its own; `claimed_approval` reports the event writer's claim, labelled as a claim. | Responsibility | Inspection |
| FR-065-CON-4 | Existing evidence-audit and assurance reports SHALL remain readable and unchanged when change assurance is not selected. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-065-AC-1 | Receipt schema v1 records the exact record/candidate, decision event and its sequence, predecessor revisions, record/lineage/review/impact checks, every proof and auditor finding, unknowns, overall outcome, and reasons; undeclared fields are refused. | Test |
| FR-065-AC-2 | Valid, invalid, and incomplete remain distinct; any invalid check dominates incomplete, any incomplete check dominates valid, and an empty reason set is permitted only for valid. | Property |
| FR-065-AC-3 | Exactly one proof receipt is emitted per reviewed proof in id order; missing selection is incomplete, duplicate or unknown-proof selection is invalid, and unselected stored attestations do not affect the result. | Property |
| FR-065-AC-4 | Changing record id or revision, candidate revision, proof id, argv/cwd, tool identity, or configuration independently makes the selected attestation invalid and names the mismatched premise. | Property |
| FR-065-AC-5 | Passed evidence can discharge only when its retained output has the declared size and all owning obligations are healthy; failed, stale, vacuous, unrelated, output-mismatched, or other defect evidence is invalid. | Property |
| FR-065-AC-6 | A missing attestation or output, unavailable or not-computed result, or audit not-evaluated state is incomplete and never valid or numeric zero. | Test |
| FR-065-AC-7 | Exactly one intact matching human `approved` event permits review validity; missing history/event is incomplete; duplicate matching decisions, a sequence gap or repeat, mismatched event, rejection, or revise request are invalid. | Integration |
| FR-065-AC-8 | Incomplete/truncated impact evidence and every non-resolved unknown remain named and force incomplete verification while preserving an advisory workflow's ability to continue. | Test |
| FR-065-AC-9 | The receipt retains each FR-032 finding kind and obligation id byte-for-byte beside its mapped reason without changing the source auditor result (CON-2). | Integration |
| FR-065-AC-10 | Input and selection permutations emit byte-identical canonical receipts, compared byte for byte. | Property |
| FR-065-AC-11 | Existing audit/assurance goldens stay byte-identical without change-assurance selection, and static boundaries prove verification runs and writes nothing (CON-1, CON-4). | Integration |
| FR-065-AC-12 | Golden terminology describes actor labels only as recorded attribution, never identity, authority, authenticity, signature, or non-repudiation, and `claimed_approval` only as a claim to be re-verified (CON-3). | Analysis |
| FR-065-AC-13 | A matching `approved` event whose `verified_approval` is `approved`, carries the record's id and the digest of the stored revision, and a `trust.allowed_signers` equal to the record's `allowed_signers_path`, keeps the review check valid, and the receipt's `claimed_approval` carries the protocol, tag, digest, trust, approver, every meaning value in order and the signature kind. | Integration |
| FR-065-AC-14 | A `verified_approval` with status `wrong_record`, or `approved` with a digest that is not the digest of the stored revision (the record changed after the tag), makes the receipt `invalid` with `approval_record_mismatch` and `claimed_approval` null. | Integration |
| FR-065-AC-15 | A `verified_approval` with status `untrusted_signer` (an unmapped signer) makes the receipt `invalid` with `approval_signer_untrusted`; `signed_bad` makes it `invalid` with `approval_signature_bad`; a `trust.allowed_signers` that differs from the record's non-null `allowed_signers_path` makes it `invalid` with `approval_signer_untrusted`. | Integration |
| FR-065-AC-16 | A `verified_approval` with status `meaning_missing` makes the receipt `incomplete` with `approval_meaning_missing`; `unsigned` or `unverifiable` makes it `incomplete` with `approval_unverified`; none of them is `valid`, and `invalid` still dominates when another check is invalid. | Integration |
| FR-065-AC-17 | Two matching `approved` events that each carry a `verified_approval` make the receipt `invalid` with `decision_mismatch`, and `claimed_approval` is null. | Integration |
| FR-065-AC-18 | An approved event with no `verified_approval` under a record whose `signed_approval.required` is false yields `claimed_approval` null and the same review check as before; a version 1 payload or receipt is not accepted; static boundaries prove the approval check starts no process and performs no Git, network or identity-provider operation (CON-1). | Test |
| FR-065-AC-19 | An approved event with no `verified_approval` under a record whose `signed_approval.required` is true makes the receipt `incomplete` with `approval_missing`, and the same record with `required` false is not affected. | Integration |
| FR-065-AC-20 | When a `signed_bad` claim also carries a digest that differs from the stored record, the receipt reports `approval_signature_bad` and not `approval_record_mismatch`, following the status order of Engineering Assurance FR-041. | Test |

## Dependencies

- **Upstream**: [FR-032](./FR-032-evidence-auditor.md),
  [FR-063](./FR-063-change-assurance-record-integrity.md),
  [FR-064](./FR-064-proof-attestation.md), and ix-flow FR-013/018.
- **Downstream**: advisory pilots and explicitly promoted assurance profiles may
  consume the outcome; this requirement defines no merge gate or promotion
  policy.
