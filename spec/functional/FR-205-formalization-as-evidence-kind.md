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

Quoin SHALL show each approved or analysed formalization as `formalization`
evidence on its requirement using the per-criterion coverage-state record and
revision-free clause-address contract owned by Contract IR FR-071. The record's
`criterion_id`, `state`, optional `clause_address`, and optional shared
Engineering Assurance `evidence` SHALL conform to FR-071. Quoin SHALL use that
contract directly and SHALL NOT define a local record protocol.

## Inputs

- The formalization-coverage items and derived states defined by [FR-204](./FR-204-formalization-coverage-state.md).
- The coverage-state record and clause-address contract in
  [Contract IR FR-071](ix://agent-ix/filament-contract-ir/FR-071).
- The qualified evidence and obligation coverage results defined by FR-153.

## Outputs

- For every Quire-authored acceptance criterion in the selected corpus,
  `quoin assurance` produces exactly one per-criterion formalization
  coverage-state record conforming to FR-071. It presents the record under its
  owning requirement. A criterion with state `none` or `draft` remains a gap in
  the assurance view; representing that state in the coverage record does not
  turn the gap into evidence.
- For an `approved` or `analysed` criterion, `quoin assurance` presents the
  corresponding formalization record as evidence under its owning requirement.
- For a `draft` item, `quoin assurance` includes the derived draft reason with the gap.
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
  vocabulary. Quoin SHALL map only states supported by its FR-204 and FR-153
  inputs; it SHALL preserve `proved`, `rejected`, and `inconclusive` as distinct
  outcomes and SHALL NOT infer an intermediate or successful state from a
  missing or unavailable producer result.
- `evidence`, when present, SHALL be the shared Engineering Assurance
  `EvidenceEnvelope` defined by FR-071. Quoin SHALL preserve the envelope as
  supplied by its authoritative producer and SHALL NOT replace it with a local
  evidence shape. The record's formalization `state` SHALL remain distinct
  from the envelope's availability state.
- Quoin SHALL consume the shared Engineering Assurance `EvidenceEnvelope`
  defined by FR-004 and FR-009 through FR-071. This requirement does not repeat
  those field definitions or define a parallel envelope or serialization
  protocol.

## Behavior

- For each Quire-authored criterion, Quoin SHALL emit one record with its exact
  FR-071 `SourceAuthoredCriterionId`, even when the derived FR-204 state is
  `none` or `draft`.
- Quoin SHALL map `none` to FR-071 `eligible`, or `not-applicable` only when
  the requirement is explicitly marked not formalizable and a non-empty
  formalization reason is available.
- Quoin SHALL map `draft` to FR-071 `authored` and retain the derived approval
  reason in the gap presentation.
- Quoin SHALL map an approved declaration checked by QSL to FR-071
  `typechecked`.
- Quoin SHALL map a declaration lowered into a contract package to FR-071
  `lowered`.
- Quoin SHALL map an approved declaration with a FR-153 `qualified-record`
  disposition and agreement `satisfied` to FR-071 `proved`.
- Quoin SHALL map a `refuted` analysis result, represented by the FR-153
  `failed` disposition, to FR-071 `rejected`, with a non-empty formalization
  reason from the authoritative analysis result.
- Quoin SHALL map a completed analysis whose outcome is inconclusive, or a
  FR-153 disposition of `errored`, `absent`, `stale`, `unevaluated`, or
  `qualification-rejected`, to FR-071 `inconclusive`, with a non-empty
  formalization reason from the authoritative disposition or result.
- Quoin SHALL preserve FR-071 `unknown` and `unavailable` when the authoritative
  input reports those states; it SHALL NOT collapse them into `inconclusive`.
  It SHALL preserve each state's required formalization reason separately from
  Engineering Assurance evidence validation errors, diagnostic category, and
  boundary rationale.
- Quoin SHALL provide a non-empty formalization reason for `not-applicable`
  from the explicit formalization exclusion. When an authoritative input reports
  `unknown` or `unavailable`, Quoin SHALL retain that input's non-empty
  formalization reason.
- Quoin SHALL emit only states supported by its authoritative inputs. The
  upstream states `parsed` and `backend_supported` remain available to their
  respective producers and SHALL NOT be inferred by Quoin.
- Quoin SHALL show each item in state `none` or `draft` as a gap.
- Quoin SHALL present a coverage-state record for a `none` or `draft` item as a
  gap, not as formalization evidence.
- Quoin SHALL NOT count a formalization evidence record toward the coverage of any Test, Analysis, or Inspection obligation.
- Quoin SHALL assign analysis-obligation credit only when the FR-153 route returns `qualified-record`.
- Formalization evidence SHALL NOT change the result of `quoin matrix` for any obligation.

## Dependencies

- **Upstream:** [FR-204](./FR-204-formalization-coverage-state.md) defines Quoin's item states; [FR-153](./FR-153-analysis-result-counting-by-disposition.md) is the only route to analysis-obligation credit; [FR-071](ix://agent-ix/filament-contract-ir/FR-071) owns the per-criterion record, identity, state vocabulary, clause address, and shared evidence-envelope consumption contract.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-205-AC-1 | Given an approved formalization for a Quire criterion, `quoin assurance` emits exactly one FR-071 record for that criterion and presents it as formalization evidence under its owning requirement. | Test |
| FR-205-AC-2 | Given an analysed formalization, the record state is `proved` only for a FR-153 `qualified-record` with agreement `satisfied`; a refuted result is `rejected`, and other completed non-success outcomes remain `inconclusive`. | Test |
| FR-205-AC-3 | Given a draft item with a derived approval reason, the record retains the criterion identity and state `authored`, while `quoin assurance` shows the reason beside the gap and does not present it as evidence. | Test |
| FR-205-AC-4 | Given an item in state `none`, the record state is `eligible` (or `not-applicable` only for an explicit exclusion with a non-empty formalization reason), and `quoin assurance` shows a gap rather than formalization evidence. | Test |
| FR-205-AC-5 | Given a corpus and obligations with or without formalization evidence, every obligation has identical coverage credit in `quoin matrix`; formalization evidence alone grants no credit to Test, Analysis, or Inspection obligations. | Test |
| FR-205-AC-6 | Given valid FR-204/FR-153 inputs and authoritative Quire rows, Quoin constructs FR-071 records and the `quoin assurance` producer/consumer path validates each constructed record before rendering it; malformed producer output (including a missing required reason or an address-presence mismatch for its state) fails the record validation and is not presented as evidence. The path does not accept arbitrary caller-supplied coverage records. | Test |
| FR-205-AC-7 | Given two Quire coverage rows whose `source`, `document`, or `id` differs, Quoin emits distinct records keyed by the corresponding full `(obligation_declaration, scoped_document, criterion_id)` tuple, even when flattened labels collide. | Test |
| FR-205-AC-8 | Given a record with a clause address, the address contains the FR-071 package identity, criterion identity, clause id, and source span; the span preserves FR-012 `SourceIdentity`, including source-document identity and revision, while the address contains no requirement revision, statement hash, digest, or file hash. | Test |
| FR-205-AC-9 | Given a criterion inventory, Quoin emits exactly one coverage-state record per criterion; `unknown` and `unavailable` remain distinct, `not-applicable` uses the canonical token, and `parsed` and `backend_supported` are never inferred without authoritative producer input. | Test |
| FR-205-AC-10 | Given each FR-071 state, Quoin omits or includes `clause_address` and `reason` exactly as FR-071 requires; a required reason is non-empty and separate from Engineering Assurance evidence validation errors, diagnostic category, and boundary rationale. | Test |
| FR-205-AC-11 | Given a producer-supplied evidence envelope, Quoin preserves it using Engineering Assurance FR-004/FR-009 semantics and leaves it absent when no producer evidence exists; it does not define or serialize a parallel envelope shape. | Test |
