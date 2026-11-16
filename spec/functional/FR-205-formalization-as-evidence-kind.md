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
  - target: "ix://agent-ix/quire-contract-ir/FR-006"
    type: "references"
---
# FR-205: Show formalization as evidence without obligation credit

## Description

Quoin SHALL show each approved or analysed formalization as `formalization`
evidence on its requirement using the coverage-state record and clause-address
contract owned by IR-96.

## Inputs

- The formalization-coverage items and derived states defined by [FR-204](./FR-204-formalization-coverage-state.md).
- The IR-96 coverage-state record and clause-address contract.
- The qualified evidence and obligation coverage results defined by FR-153.

## Outputs

- `quoin assurance` shows each eligible formalization as evidence on its requirement and each `none` or `draft` item as a gap.
- For a `draft` item, `quoin assurance` includes the derived draft reason with the gap.
- Each formalization evidence record conforms to the IR-96 contract; this requirement does not define or alter that record's shape.

## Behavior

- Quoin SHALL show each item in state `approved` or `analysed` as `formalization` evidence on the corresponding requirement, conforming to the coverage-state record and clause-address contract owned by IR-96.
- Quoin SHALL map `none` to IR-96 `eligible`, or `not-applicable` when the requirement is marked not formalizable.
- Quoin SHALL map `draft` to IR-96 `authored` and retain the derived approval reason.
- Quoin SHALL map an approved declaration checked by QSL to at least IR-96 `typechecked`.
- Quoin SHALL map a declaration lowered into a contract package to IR-96 `lowered`.
- Quoin SHALL map an approved declaration with a FR-153 `qualified-record` disposition and agreement `satisfied` to IR-96 `proved`.
- Quoin SHALL map a `refuted` analysis result, represented by the FR-153 `failed` disposition, to IR-96 `rejected`.
- Quoin SHALL map `inconclusive`, `unavailable`, `errored`, `absent`, `stale`, `unevaluated`, and `qualification-rejected` analysis dispositions to IR-96 `inconclusive`.
- Quoin SHALL emit only the IR-96 states `eligible`, `not-applicable`, `authored`, `typechecked`, `lowered`, `proved`, `rejected`, and `inconclusive` for this lane; other IR-96 states remain owned by their respective producers.
- Quoin SHALL show each item in state `none` or `draft` as a gap.
- Quoin SHALL NOT emit a formalization evidence record for any item in state `none` or `draft`.
- Quoin SHALL NOT count a formalization evidence record toward the coverage of any Test, Analysis, or Inspection obligation.
- Quoin SHALL assign analysis-obligation credit only when the FR-153 route returns `qualified-record`.
- Formalization evidence SHALL NOT change the result of `quoin matrix` for any obligation.

## Dependencies

- **Upstream:** [FR-204](./FR-204-formalization-coverage-state.md) defines item states; [FR-153](./FR-153-analysis-result-counting-by-disposition.md) is the only route to analysis-obligation credit; [IR-96](ix://agent-ix/quire-contract-ir/FR-006) owns the coverage-state record and clause-address contract.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-205-AC-1 | Given an item in state `approved`, `quoin assurance` shows one formalization evidence record on its requirement, conforming to IR-96, and no locally defined substitute record shape is emitted. | Test |
| FR-205-AC-2 | Given an item in state `analysed`, `quoin assurance` shows its formalization evidence with the corresponding IR-96 state for its QSL, lowering, and qualified-analysis progress. | Test |
| FR-205-AC-3 | Given a draft item with a derived approval reason, `quoin assurance` shows the reason beside the gap and emits no formalization evidence record for that item. | Test |
| FR-205-AC-4 | Given an item in state `none`, `quoin assurance` shows a gap and emits no formalization evidence record for that item. | Test |
| FR-205-AC-5 | Given a corpus and obligations with or without formalization evidence, every obligation has identical coverage credit in `quoin matrix`; formalization evidence alone grants no credit to Test, Analysis, or Inspection obligations. | Test |
| FR-205-AC-6 | Given a record that does not conform to the IR-96 coverage-state contract or clause address, Quoin rejects it and does not show it as formalization evidence. | Test |
