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
  NOT reconstruct that identity from a flattened criterion id or a generated
  label.
- `clause_address`, when present, SHALL use exactly FR-071's
  `package_identity`, `criterion_id`, `clause_id`, and `source_span` fields.
  The span's start and end locations SHALL retain the FR-071 `SourceIdentity`
  exactly as FR-012 defines it, including source-document identity and
  source-document revision, as well as one-based line and column and byte
  offset. This source-document revision is part of `source_span`; it is not a
  requirement revision member of the address. The address SHALL contain no
  requirement revision, statement hash, digest, or file hash.
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
- The FR-071 `EvidenceEnvelope` has the shared fields
  `producer_id`, `availability`, `observation`, `governing`, `next_action`,
  `owner`, `boundary_rationale`, `quoin_reference`, and `validation_errors`.
  Quoin consumes the Engineering Assurance definitions of these members; this
  requirement does not define a parallel envelope or serialization protocol.

## Behavior

- For each Quire-authored criterion, Quoin SHALL emit one record with its exact
  FR-071 `SourceAuthoredCriterionId`, even when the derived FR-204 state is
  `none` or `draft`.
- Quoin SHALL map `none` to FR-071 `eligible`, or `not_applicable` only when
  the requirement is explicitly marked not formalizable.
- Quoin SHALL map `draft` to FR-071 `authored` and retain the derived approval
  reason in the gap presentation.
- Quoin SHALL map an approved declaration checked by QSL to FR-071
  `typechecked`.
- Quoin SHALL map a declaration lowered into a contract package to FR-071
  `lowered`.
- Quoin SHALL map an approved declaration with a FR-153 `qualified-record`
  disposition and agreement `satisfied` to FR-071 `proved`.
- Quoin SHALL map a `refuted` analysis result, represented by the FR-153
  `failed` disposition, to FR-071 `rejected`.
- Quoin SHALL map a completed analysis whose outcome is inconclusive, or a
  FR-153 disposition of `errored`, `absent`, `stale`, `unevaluated`, or
  `qualification-rejected`, to FR-071 `inconclusive`.
- Quoin SHALL preserve FR-071 `unknown` and `unavailable` when the authoritative
  input reports those states; it SHALL NOT collapse them into `inconclusive`.
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
| FR-205-AC-4 | Given an item in state `none`, the record state is `eligible` (or `not_applicable` only for an explicit exclusion), and `quoin assurance` shows a gap rather than formalization evidence. | Test |
| FR-205-AC-5 | Given a corpus and obligations with or without formalization evidence, every obligation has identical coverage credit in `quoin matrix`; formalization evidence alone grants no credit to Test, Analysis, or Inspection obligations. | Test |
| FR-205-AC-6 | Given a record that does not conform to FR-071's criterion identity, state, clause address, or shared evidence envelope, Quoin rejects it and does not show it as formalization evidence. | Test |
| FR-205-AC-7 | Given two criteria that share a flattened id but differ in any component of `SourceAuthoredCriterionId`, Quoin emits distinct records with the exact Quire-authored identities and does not merge or relabel them. | Test |
| FR-205-AC-8 | Given a record with a clause address, the address contains the FR-071 package identity, criterion identity, clause id, and start/end source span; `source_span` preserves its FR-012 `SourceIdentity`, including source-document identity and revision, while the address contains no requirement revision, statement hash, digest, or file hash. | Test |
| FR-205-AC-9 | Given a criterion inventory, Quoin emits exactly one coverage-state record per criterion; `unknown` and `unavailable` remain distinct, while `parsed` and `backend_supported` are never inferred without authoritative producer input. | Test |
| FR-205-AC-10 | Given an evidence envelope, the record preserves the shared Engineering Assurance value and its availability independently of the formalization state; when no producer evidence exists, the optional envelope is absent. | Test |
