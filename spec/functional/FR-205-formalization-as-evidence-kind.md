---
id: FR-205
title: "Show formalization as evidence without obligation credit"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-204"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-153"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-184"
    type: "references"
  - target: "ix://agent-ix/filament-contract-ir/FR-071"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: "references"
---
# FR-205: Show formalization as evidence without obligation credit

## Description

Contract IR produces the per-criterion coverage-state records owned by FR-071.
When `quoin assurance --formalization-records <FILE>` is supplied, Quoin SHALL
consume the file's top-level JSON array of FR-071 record values, validate those
values against FR-071, and present them under their owning requirements. The
file is transport for the existing FR-071 records; Quoin SHALL NOT produce,
wrap, or define a local record protocol.

## Inputs

- The formalization-coverage items and derived states defined by [FR-204](./FR-204-formalization-coverage-state.md).
- The coverage-state record and clause-address contract in
  [Contract IR FR-071](ix://agent-ix/filament-contract-ir/FR-071).
- When the option is supplied, a file whose top-level JSON value is an array of
  the exact FR-071 per-criterion record values. Contract IR owns production of
  these records; the array adds no record fields or wrapper.
- The qualified evidence and obligation coverage results defined by FR-153.

## Outputs

- When the file is supplied, `quoin assurance` presents each matched FR-071
  record under its owning requirement. Quoin does not produce or modify the
  record value.
- When the file is not supplied, `quoin assurance` does not display
  FR-071-based formalization records.
- A record in state `eligible`, `authored`, `unknown`, `unavailable`, or
  `not-applicable` is shown as a gap. Records in other valid states are shown
  in the separate formalization evidence view, retaining their FR-071 state and
  any required reason. Neither presentation grants obligation coverage credit.
- The FR-071 state determines gap or evidence presentation. An FR-204 state of
  `approved` or `analysed` SHALL NOT route an FR-071 gap state into the evidence
  view.
- For an FR-204 item in state `draft` matched to an FR-071 `authored` record,
  `quoin assurance` shows the derived draft reason from FR-204 beside the gap.
- `criterion_id` SHALL preserve the exact Quire-authored identity from FR-071:
  `obligation_declaration`, `scoped_document`, and `criterion_id`. Quoin SHALL
  obtain those components directly from the authoritative Quire coverage row's
  `source`, `document`, and `id` fields respectively, and SHALL join records by
  the complete three-part identity. Quoin SHALL NOT reconstruct the identity
  from a flattened criterion id or generated label.
- `clause_address`, when present, SHALL use exactly FR-071's
  `package_identity`, `criterion_id`, `clause_id`, and `source_span` fields;
  its `criterion_id` SHALL equal the record's full criterion identity.
  The span's start and end locations SHALL retain the FR-071 `SourceIdentity`
  exactly as FR-012 defines it, including source-document identity and
  source-document revision. This source-document revision is part of
  `source_span`; it is not a requirement-revision member of the address. The
  address SHALL contain no requirement revision, statement hash, digest, or
  file hash.
- The address presence SHALL follow FR-071's state rules: `eligible` and
  `not-applicable` omit `clause_address`; `authored`, `parsed`, `typechecked`,
  `lowered`, `backend_supported`, `proved`, `rejected`, and `inconclusive`
  include it; `unknown` and `unavailable` include it exactly when a formal
  clause exists in the selected package.
- The FR-071 `reason` member SHALL be non-empty for `rejected`, `inconclusive`,
  `unknown`, `unavailable`, and `not-applicable`, and SHALL be absent for
  `eligible`, `authored`, `parsed`, `typechecked`, `lowered`,
  `backend_supported`, and `proved`. A draft approval reason shown with the
  `authored` gap remains Quoin's derived gap detail, not the FR-071
  formalization `reason` member.
- The record's `state` SHALL be one value from FR-071's `FormalizationState`
  vocabulary. The supplied FR-071 state is authoritative; Quoin SHALL preserve
  it and SHALL NOT derive or replace it from FR-204 or FR-153 inputs.
- `evidence`, when present, SHALL be the shared Engineering Assurance
  `EvidenceEnvelope` defined by FR-071. Quoin SHALL preserve the envelope as
  supplied by its authoritative producer and SHALL NOT replace it with a local
  evidence shape. The record's formalization `state` SHALL remain distinct
  from the envelope's availability state.
- Quoin SHALL consume the shared Engineering Assurance `EvidenceEnvelope`
  defined by FR-004 and FR-009 through FR-071. This requirement does not repeat
  those field definitions or define a parallel envelope or serialization
  protocol.
- Contract IR remains the sole producer of FR-071 records. Quoin SHALL consume
  the supplied values and SHALL NOT construct or rewrite `criterion_id`,
  `state`, `clause_address`, `reason`, or `evidence`.

## Behavior

- When the option is supplied, Quoin SHALL parse the file as a top-level JSON
  array of FR-071 records, validate each record under FR-071, and reject the
  input if any record is invalid. The transport SHALL NOT add a wrapper,
  protocol identifier, or Quoin-owned record members.
- Quoin SHALL require exactly one record for each criterion in the selected
  Quire inventory. Quoin SHALL reject missing, duplicate, or extra records
  without rendering any supplied formalization record.
- When Quoin matches a record to a Quire coverage row, it SHALL read the
  complete FR-071 `SourceAuthoredCriterionId` directly from row `source`
  (`obligation_declaration`), `document` (`scoped_document`), and `id`
  (`criterion_id`).
- Quoin SHALL join records using all three `SourceAuthoredCriterionId` values.
- Quoin SHALL NOT join records by a flattened id, label, or display text.
- When Quoin validates and matches a record, it SHALL present the record under
  the requirement that owns the matched Quire criterion.
- Quoin SHALL preserve every supplied FR-071 member as defined by that
  contract.
- Quoin SHALL NOT synthesize replacement values for an FR-071 record.
- When the matched FR-204 item state is `none`, the FR-071 record state SHALL
  be `eligible`, or `not-applicable` only for an explicit FR-204 exclusion
  with a non-empty FR-071 reason. Quoin SHALL show either valid record as a gap.
- When the matched FR-204 item state is `draft`, the FR-071 record state SHALL
  be `authored`. Quoin SHALL show it as a gap and display the derived approval
  reason from FR-204. The FR-071 `reason` member remains absent for `authored`.
- Quoin SHALL reject the input without rendering formalization records when an
  FR-204 `none` item is paired with any FR-071 state other than `eligible` or a
  justified `not-applicable`, or an FR-204 `draft` item is paired with any
  state other than `authored`.
- When Quoin matches a record to an FR-204 item in state `approved` or
  `analysed`, it SHALL present an FR-071 gap state as a gap. FR-204 `none` and
  `draft` pairings follow the explicit state-alignment rules above.
- Quoin SHALL present only non-gap FR-071 states in the separate formalization
  evidence view, retaining the supplied state and any required reason. This
  includes preserving `rejected` and `inconclusive` as distinct states.
- Quoin SHALL NOT change the result or coverage fields of `quoin matrix` when it
  displays any formalization record.
- Quoin SHALL present `unknown`, `unavailable`, and `not-applicable` as
  stateful gaps using their supplied non-empty FR-071 `reason`; it SHALL keep
  those reasons distinct from Engineering Assurance validation errors,
  diagnostic category, and boundary rationale.
- Quoin SHALL NOT count a formalization evidence record toward the coverage of any Test, Analysis, or Inspection obligation.
- Quoin SHALL assign analysis-obligation credit only when the FR-153 route returns `qualified-record`.
- Formalization evidence SHALL NOT change the result of `quoin matrix` for any obligation.

## Dependencies

- **Upstream:** [FR-204](./FR-204-formalization-coverage-state.md) defines Quoin's item states; [FR-153](./FR-153-analysis-result-counting-by-disposition.md) is the only route to analysis-obligation credit; [FR-071](ix://agent-ix/filament-contract-ir/FR-071) owns the per-criterion record, identity, state vocabulary, clause address, and shared evidence-envelope consumption contract.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-205-AC-1 | Given a file with exactly one valid FR-071 record for each selected Quire criterion, `quoin assurance --formalization-records <FILE>` presents each matched record under its owning requirement without changing the record value. | Test |
| FR-205-AC-2 | Given a valid FR-071 record in a formalization state other than `eligible`, `authored`, `unknown`, `unavailable`, or `not-applicable`, `quoin assurance` presents the supplied state and any required reason in the separate formalization evidence view without deriving a replacement state from FR-204 or FR-153. | Test |
| FR-205-AC-3 | Given a Quire criterion whose FR-204 item state is `draft` and a matched FR-071 record in state `authored`, `quoin assurance` shows a gap with the FR-204 draft reason; the supplied FR-071 record remains unchanged and has no `reason` member. | Test |
| FR-205-AC-4 | Given a Quire criterion whose FR-204 item state is `none` and a matched FR-071 record in state `eligible` or justified `not-applicable`, `quoin assurance` shows a gap; `not-applicable` retains its required FR-071 reason. | Test |
| FR-205-AC-5 | Given a corpus and obligations with or without formalization evidence, every obligation has identical coverage credit in `quoin matrix`; formalization evidence alone grants no credit to Test, Analysis, or Inspection obligations. | Test |
| FR-205-AC-6 | Given a supplied `--formalization-records` file with malformed JSON, a record that violates FR-071 (including its state-dependent `reason` or `clause_address` rule), an FR-204 `none`/`draft` state mismatch, a missing/duplicate/extra criterion record, or a record that cannot be joined to the Quire inventory, Quoin rejects the input and presents no formalization records. | Test |
| FR-205-AC-7 | Given a record in state `eligible`, `authored`, `unknown`, `unavailable`, or `not-applicable` and an FR-204 item state `approved` or `analysed`, `quoin assurance` still renders the record as a gap and does not route it into formalization evidence. | Test |
| FR-205-AC-8 | Given two Quire coverage rows whose `source`, `document`, or `id` differs, the input records join only by their full `(obligation_declaration, scoped_document, criterion_id)` tuples, even when flattened labels collide. | Test |
| FR-205-AC-9 | Given a record with a clause address, the address contains the FR-071 package identity, criterion identity, clause id, and source span; the span preserves FR-012 `SourceIdentity`, including source-document identity and revision, while the address contains no requirement revision, statement hash, digest, or file hash. | Test |
| FR-205-AC-10 | Given a consumed record, Quoin enforces FR-071's state-dependent `clause_address` and `reason` presence rules, including non-empty reasons for `rejected`, `inconclusive`, `unknown`, `unavailable`, and `not-applicable`. | Test |
| FR-205-AC-11 | Given a consumed record with an evidence envelope, Quoin validates and preserves the shared Engineering Assurance FR-004/FR-009 value; it does not define, produce, or serialize a parallel envelope shape. | Test |
