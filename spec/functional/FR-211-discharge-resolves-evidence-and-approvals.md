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
  - target: "ix://agent-ix/quoin/FR-098"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "depends_on"
  - target: "ix://agent-ix/quire-rs/FR-073"
    type: "depends_on"
---

# FR-211: Clause discharge resolves its evidence and verifies its approvals

## Description

Quoin SHALL discharge a clause only on evidence and approval that resolve at the
moment a discharge report is built or rendered. Quoin SHALL resolve each
`evidenceRefs` entry of a direct fact to a retained record in the evidence store
whose kind satisfies one of the clause's `expectedOutputs`. For a clause that
declares `approvalRoles` (an approval-class clause), Quoin SHALL resolve the
fact's `approvalRef` to a signed approval that engineering-assurance verifies.
When a clause declares no `approvalRoles`, Quoin SHALL keep `approvalRef` as a
caller-supplied reference (FR-046).

Quoin records who verifiably approved. It does not decide risk acceptance:
people accept risk, and the tool assigns no score, class or acceptability.

## Inputs

- A discharge fact, the validated `clause-binding-v1` report and the installed
  modules that declare its clause set (FR-208).
- The evidence store of the repository (FR-030).
- For an approval-class clause: `--repo <dir>`, `--engineering-assurance <abs path>`,
  `--allowed-signers <abs path>` and the optional gitsign trio, with the
  meaning and validation FR-188 gives them.

## Behavior

### Evidence references

- An `evidenceRefs` entry is `evidence://<path>`, where `<path>` is the
  store-relative path of one retained record without its `.json` suffix:
  `<family>/<SUITE-N>/<stem>` for `runs`, `scans`, `mock-inspections` and
  `analysis-reports`; `experiments/<record-id>` and `operational/<record-id>`;
  and `scenarios/<integration>/<scenario>`. These families are the only ones a
  fact may cite.
- Any other ref is `unresolved`: an unknown or non-citable family (including
  `discharge`, `trust` and `bindings.json`), another scheme, an empty segment, a
  `.` or `..` segment, or a path leaving the store root.
- Segments are matched exactly as written. The `integration` and `scenario`
  segments are compared in the store's own folded (lowercase) form, so a ref
  whose scenario segment is not in that form does not resolve; no two spellings
  alias one record.
- A ref resolves when the store holds a readable record at that path. Quoin
  reads the record file through the evidence store reader of `quoin-evidence`.
- The retained kind of a record is its `subject.kind` for an experiment or
  operational record, and otherwise the family's singular name: `run`, `scan`,
  `mock-inspection`, `analysis-report`, `scenario`.
- Each `expectedOutputs` entry is a name in the `outputCatalog` of the uniquely
  resolved clause set that defines the clause (FR-208). Quoin reads one catalog
  member, `producers`, from each catalog entry: a non-empty list of retained
  kinds that satisfy the output. A resolved record satisfies the clause when its
  kind is in the `producers` of at least one of the clause's `expectedOutputs`.
  Otherwise the ref has reason `kind-not-expected`. A clause with empty
  `expectedOutputs` accepts a resolved ref of any kind.
- Until the catalog schema carries `producers` (quire-rs FR-073), no catalog
  entry has any, so a direct fact on a clause with non-empty `expectedOutputs`
  has reason `no-producer` and does not discharge.

### Approval verification

- A fact on an approval-class clause with no `approvalRef`, or a blank one,
  leaves the clause open with reason `approval-missing`, and Quoin makes no
  engineering-assurance call.
- Otherwise Quoin SHALL invoke engineering-assurance `approval-verify` (its
  FR-041) with the request members `protocol`
  (`engineering-assurance.approval-verify-request/v1`), `repository` (the
  absolute path of `--repo`), `tag` (the fact's `approvalRef`), `record_id`,
  `record_digest`, `allowed_signers` and, when supplied, `gitsign`.
- `record_id` is `<authority>/<set id>/<version>/<clauseId>`. `record_digest` is
  `sha256:` followed by the lowercase hex SHA-256 of the RFC 8785 canonical JSON
  (as FR-098 specifies, from `quire-canonical`) of the fact with its
  `approvalRef` member removed. The approval is a signed annotated tag whose
  message carries those two values as trailers.
- A `record_id` longer than 256 bytes has reason `record-id-too-long`; Quoin
  makes no engineering-assurance call and never reports it as unavailable.
- Quoin SHALL accept the approval only when the result's `status` is `approved`
  and its `meaning` array holds exactly one value that equals, byte for byte, one
  of the clause's `approvalRoles`. Any other status, or any other meaning, has
  reason `approval-unverified` naming the status or the roles.
- The verified approver is the `approver` of the result. For a direct fact on an
  approval-class clause, `attestation.attestedBy` SHALL equal it, or the reason
  is `attester-not-approver`.
- Refusals map as FR-188 maps them: an allowed-signers file engineering-assurance
  refuses is a usage error with exit 2; engineering-assurance or git unavailable,
  or any other refusal, has exit 3 where a command exits and reason
  `approval-verifier-unavailable` where a report is built. Quoin SHALL NOT
  substitute a syntactic check for the verification.

### Customer-owned clauses

- A clause is customer-owned when its `expectedOutputs` is non-empty and every
  entry has `kind: procedure` in the output catalog. A direct fact for it has
  reason `customer-owned`; a disposition for it discharges only when its decision
  is `delegated` and its rationale and approval reference are non-blank.

### When resolution and verification run

- Quoin SHALL run evidence resolution and approval verification every time it
  builds or renders a discharge report from facts: `quoin discharge` with
  `--facts` or `--repo`, `--inward`, the library report builders, and `quoin
  evidence record-discharge`.
- A rendering operation given an already-built report (including a report
  document on stdin, as `render_discharge` and `build_authored_argument` take)
  SHALL re-run both checks over every direct fact's refs and every approval-class
  approval it carries, and report a failing clause as open.
- A clause whose check fails, whatever the cause, including a ref whose record
  `quoin evidence gc` has since removed, is not discharged in that report: it is
  open with the reason above, and the report names the ref or `approvalRef`.
  Facts read from committed files under `discharge/` are checked the same way.
- The report shows the approver verified by the current verification. The
  store keeps no verification result and no approver (FR-133), and a stored
  `attestedBy` is never labelled verified.
- `quoin evidence record-discharge` additionally refuses the whole list, writes
  nothing and exits non-zero when any fact fails these checks; an unavailable
  verifier exits 3.

### Approval subject

- `quoin evidence approval-subject --binding <path|-> --fact <path|->` SHALL print
  one JSON line `{"record_id":"…","record_digest":"sha256:…"}` for the fact,
  computed as above against the binding report's clause set, so a person can
  create the matching approval tag. It writes no file and exits 0. A fact whose
  clause is not in the binding report, a malformed fact, or a `record_id` over
  256 bytes exits non-zero with no stdout.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-211-CON-1 | Quoin SHALL NOT compute a risk score, apply a risk threshold or decide risk acceptance from a verified approval; the approver's signature is evidence that a named person approved. | Security | Inspection |
| FR-211-CON-2 | Quoin SHALL NOT verify a signature itself; it relies only on the engineering-assurance `approval-verify` result. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-211-AC-1 | A direct fact whose every `evidenceRefs` entry resolves to a retained record of a kind in the `producers` of one of the clause's `expectedOutputs` discharges; a clause with empty `expectedOutputs` discharges on a resolved ref of any kind. | Test |
| FR-211-AC-2 | A ref naming no retained record, a non-citable family (`discharge`, `trust`, `bindings.json`), an unknown family, another scheme, an empty, `.` or `..` segment, or a path outside the store is `unresolved`, and the clause is open naming the ref. | Test |
| FR-211-AC-3 | A ref that resolves to a record whose kind is in no `producers` of the clause's `expectedOutputs` has reason `kind-not-expected`, and the clause is open. | Test |
| FR-211-AC-4 | A `scenarios/` ref whose scenario segment is not in the store's folded form is `unresolved`. | Test |
| FR-211-AC-5 | A direct fact on a clause with non-empty `expectedOutputs` whose catalog entries lack `producers` has reason `no-producer` and does not discharge. | Test |
| FR-211-AC-6 | A fact on an approval-class clause whose tag `approval-verify` reports `approved`, whose single `meaning` equals one of the clause's `approvalRoles` and whose digest matches, discharges, and its report row carries the verified approver. | Test |
| FR-211-AC-7 | A fact on an approval-class clause is open, naming the `approvalRef` and the reason, when the status is `unsigned`, `signed_bad`, `untrusted_signer`, `wrong_record`, `meaning_missing` or `unverifiable`; when the `meaning` is not exactly one value equal to a role (a value merely containing a role, or an extra value, fails); or when the fact's content differs from the one the tag approved. | Test |
| FR-211-AC-8 | A direct fact on an approval-class clause whose `attestation.attestedBy` differs from the verified approver is open with reason `attester-not-approver`. | Test |
| FR-211-AC-9 | A fact on an approval-class clause with a missing or blank `approvalRef` is open with reason `approval-missing` and engineering-assurance is not invoked. | Test |
| FR-211-AC-10 | A `record_id` over 256 bytes has reason `record-id-too-long`, engineering-assurance is not invoked, and the reason is never `approval-verifier-unavailable`. | Test |
| FR-211-AC-11 | An allowed-signers file engineering-assurance refuses exits 2; an absent or failing engineering-assurance exits 3 from `record-discharge` and leaves the clause open with `approval-verifier-unavailable` in a report; neither writes a fact. A fact on a clause with empty `approvalRoles` keeps `approvalRef` as an unverified reference and never invokes engineering-assurance. | Test |
| FR-211-AC-12 | A direct fact for a customer-owned clause has reason `customer-owned`; a `delegated` disposition with non-blank rationale and approval reference discharges; another decision or a blank rationale does not. | Test |
| FR-211-AC-13 | `quoin discharge --facts`, `--repo`, `--inward`, a report document given to render or authored-argument building, and a fact file committed under `discharge/` each run both checks: an approval-class fact with an unverified non-blank `approvalRef` is open on every one of those paths. | Test |
| FR-211-AC-14 | After `quoin evidence gc` removes a record a stored fact cites, the next report has that clause open with `unresolved` naming the ref; the stored fact file is unchanged. | Test |
| FR-211-AC-15 | `record-discharge` over a list holding one failing fact refuses the whole list, writes nothing and exits non-zero naming the clause and reason. | Test |
| FR-211-AC-16 | The report shows the approver from the current verification; a stored `attestedBy` that was not verified is not labelled verified. | Test |
| FR-211-AC-17 | `quoin evidence approval-subject` prints `record_id` and the RFC 8785 `record_digest`, and a tag created with them verifies as `approved` in AC-6; a fact with a different `approvalRef`-less content yields a different digest, and a changed `approvalRef` yields the same digest. | Test |
| FR-211-AC-18 | `approval-subject` exits non-zero with empty stdout for a clause absent from the binding report, a malformed fact, or a `record_id` over 256 bytes. | Test |
| FR-211-AC-19 | The report never states that the tool accepted a risk, and carries no score. | Inspection |

## Dependencies

- FR-046 supplies the facts; FR-208 supplies `approvalRoles` and the clause-set
  resolution; FR-030 and FR-133 supply the store; FR-188 supplies the
  engineering-assurance invocation inputs and refusal mapping; FR-098 supplies
  RFC 8785 canonicalisation.
- Engineering Assurance FR-041 supplies the `approval-verify` result.
- quire-rs FR-073 supplies the clause-set catalog and gains the `producers`
  member.
