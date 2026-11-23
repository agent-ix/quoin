---
id: FR-211
title: "Clause discharge resolves its evidence and verifies its approvals"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-046"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-208"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-133"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "depends_on"
---

# FR-211: Clause discharge resolves its evidence and verifies its approvals

## Description

Quoin SHALL admit a discharge fact only when the evidence and the approval it
names are real. A direct fact's `evidenceRefs` SHALL each resolve to a retained
record in the evidence store whose kind the clause expects. For a clause that
declares `approvalRoles` (an approval-class clause), the fact's `approvalRef`
SHALL resolve to a signed approval that engineering-assurance verifies. For a
clause that declares none, Quoin SHALL keep `approvalRef` as a caller-supplied
reference (FR-046).

Quoin records who verifiably approved. It does not decide that a risk is
acceptable: people accept risk, and the tool assigns no score, class or
acceptability.

## Inputs

- A discharge fact, the validated `clause-binding-v1` report and the installed
  modules that declare its clause set (FR-208).
- The evidence store of the repository (FR-030).
- For an approval-class clause: `--engineering-assurance <abs path>`,
  `--allowed-signers <abs path>` and the optional gitsign trio, with the
  meaning and validation FR-188 gives them.

## Behavior

- An `evidenceRefs` entry is `evidence://<path>`, where `<path>` is the
  store-relative path of one retained record without its `.json` suffix:
  `<family>/<SUITE-N>/<stem>` for the `runs`, `scans`, `mock-inspections` and
  `analysis-reports` families; `experiments/<record-id>` and
  `operational/<record-id>` for the assurance record families; and
  `scenarios/<integration>/<scenario>`. Any other family, an empty segment, a
  `.` or `..` segment, or a path that leaves the store root is unresolved.
  Discharge facts, the binding graph and trust decisions are not evidence a
  fact may cite.
- The retained kind of a record is its `subject.kind` for an experiment or
  operational record, and otherwise the family's singular name: `run`, `scan`,
  `mock-inspection`, `analysis-report`, `scenario`.
- When a direct fact is admitted, Quoin SHALL resolve every `evidenceRefs`
  entry against the store (FR-030, FR-133 lookup), reading only retained
  records. A ref that does not resolve, or whose retained kind is not an entry
  of the clause's `expectedOutputs`, refuses the whole fact: nothing is
  recorded, the exit status is non-zero, and the message names the ref and the
  reason (`unresolved` or `kind-not-expected`). A clause whose `expectedOutputs`
  is empty accepts a resolved ref of any kind.
- `expectedOutputs` is read from the entry of the uniquely resolved clause set
  that defines the clause (FR-208), not from the fact.
- When a fact on an approval-class clause is admitted, Quoin SHALL invoke the
  engineering-assurance `approval-verify` capability (its FR-041) with
  `tag` set to the fact's `approvalRef`, `record_id` set to
  `<authority>/<set id>/<version>/<clauseId>`, and `record_digest` set to
  `sha256:` followed by the SHA-256 of the canonical JSON of the fact with its
  `approvalRef` member removed. The `approvalRef` of a direct or disposition
  fact on such a clause is therefore the name of an annotated tag whose
  message carries those two trailers.
- Quoin SHALL accept the approval only when the result's `status` is
  `approved` and its `meaning` array contains at least one entry equal to one of
  the clause's `approvalRoles`. Any other status, or a meaning naming none of
  the roles, refuses the fact; the message names the `approvalRef`, the
  engineering-assurance status or the missing role, and nothing is recorded.
- The verified `approver` is the named person. For a direct fact on an
  approval-class clause, `attestation.attestedBy` SHALL equal it, and the
  report SHALL show the verified approver beside the retained `approvalRef`.
- When engineering-assurance cannot be run or refuses the request, Quoin SHALL
  fail with the FR-188 unavailable exit status and write no fact. Quoin SHALL
  NOT substitute a syntactic check for the verification.
- A clause is customer-owned when its `expectedOutputs` is non-empty and every
  entry has `kind: procedure` in the clause set's output catalog. Quoin SHALL
  refuse a direct fact for a customer-owned clause, and SHALL accept a
  disposition for it only when its decision is `delegated` and its rationale
  and approval reference are non-blank.
- Verification and resolution happen when a fact is admitted. A stored fact is
  not re-verified when it is read; the store keeps no verification result
  (FR-133).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-211-CON-1 | Quoin SHALL NOT compute a risk score, apply a risk threshold or decide risk acceptance from a verified approval; the approver's signature is evidence that a named person approved. | Security | Inspection |
| FR-211-CON-2 | Quoin SHALL NOT verify a signature itself; it relies only on the engineering-assurance `approval-verify` result. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-211-AC-1 | A direct fact whose every `evidenceRefs` entry resolves to a retained record of a kind in the clause's `expectedOutputs` is admitted; a clause with empty `expectedOutputs` admits a resolved ref of any kind. | Test |
| FR-211-AC-2 | A direct fact with a ref naming no retained record, an unknown family, an empty or `..` segment, or a path outside the store is refused with a non-zero exit, a message naming the ref and `unresolved`, and nothing recorded. | Test |
| FR-211-AC-3 | A direct fact citing a resolved record whose retained kind is not an entry of the clause's `expectedOutputs`, or a discharge-fact or trust-decision path, is refused with a message naming the ref and `kind-not-expected`, and nothing is recorded; a list with one bad ref among good ones records none of them. | Test |
| FR-211-AC-4 | A fact on an approval-class clause whose `approvalRef` engineering-assurance verifies as `approved`, with a meaning equal to one of the clause's `approvalRoles` and a matching digest, is admitted and its report row carries the verified approver. | Test |
| FR-211-AC-5 | A fact on an approval-class clause is refused, naming the `approvalRef` and the reason, when the status is `unsigned`, `signed_bad`, `untrusted_signer`, `wrong_record`, `meaning_missing` or `unverifiable`, when the meaning names none of the roles, or when the fact's content differs from the one the tag approved. | Test |
| FR-211-AC-6 | A direct fact on an approval-class clause whose `attestation.attestedBy` differs from the verified approver, including a bare person-shaped reference never verified, is refused. | Test |
| FR-211-AC-7 | When engineering-assurance is absent or refuses the request, an approval-class fact fails with the unavailable exit status and nothing is recorded; a fact on a clause with empty `approvalRoles` keeps its `approvalRef` as an unverified reference and never invokes engineering-assurance. | Test |
| FR-211-AC-8 | A direct fact for a customer-owned clause is refused; a `delegated` disposition with a non-blank rationale and approval reference is accepted; a disposition with another decision or a blank rationale is refused. | Test |
| FR-211-AC-9 | A GDPR data-protection-impact-assessment clause and an AI-Act human-oversight clause that declare `approvalRoles` are discharged only through the verified path of AC-4 to AC-6: a fact carrying an unverifiable named-person reference is refused and the clause stays open. | Test |
| FR-211-AC-10 | The discharge report, with the approval-class fact admitted, never states that the tool accepted a risk, and reports no score. | Inspection |

## Dependencies

- FR-046 supplies the facts; FR-208 supplies `approvalRoles` and the clause-set
  resolution; FR-030 and FR-133 supply the store; FR-188 supplies the
  engineering-assurance invocation inputs.
- Engineering Assurance FR-041 supplies the `approval-verify` result.
