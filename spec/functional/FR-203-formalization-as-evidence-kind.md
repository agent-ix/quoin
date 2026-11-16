---
id: FR-203
title: "Show formalization as evidence without obligation credit"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-153"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-184"
    type: "references"
  - target: "ix://agent-ix/quire-contract-ir/FR-006"
    type: "references"
---
# FR-203: Show formalization as evidence without obligation credit

## Description

Quoin SHALL show each approved or analysed formalization as `formalization`
evidence on its requirement using the coverage-state record and clause-address
contract owned by IR-96.

## Inputs

- The formalization-coverage items and derived states defined by FR-202.
- The IR-96 coverage-state record and clause-address contract.
- The qualified evidence and obligation coverage results defined by FR-153.

## Outputs

- `quoin assurance` shows each eligible formalization as evidence on its requirement and each `none` or `draft` item as a gap.
- Each formalization evidence record conforms to the IR-96 contract; this requirement does not define or alter that record's shape.

## Behavior

- Quoin SHALL show each item in state `approved` or `analysed` as `formalization` evidence on the corresponding requirement, conforming to the coverage-state record and clause-address contract owned by IR-96.
- Quoin SHALL map approval and analysis progress onto the states defined by IR-96. An approved declaration checked by QSL is at least `typechecked`; a declaration lowered into a contract package is `lowered`; a `qualified-record` analysis with agreement `satisfied` is `proved`; a `refuted` result is `rejected`; and inconclusive or unavailable analysis is `inconclusive`.
- Quoin SHALL show each item in state `none` or `draft` as a gap.
- Quoin SHALL NOT emit a formalization evidence record for any item in state `none` or `draft`.
- Quoin SHALL NOT count a formalization evidence record toward the coverage of any Test, Analysis, or Inspection obligation.
- Quoin SHALL assign analysis-obligation credit only when the FR-153 route returns `qualified-record`.
- Formalization evidence SHALL NOT change the result of `quoin matrix` for any obligation.

## Dependencies

- **Upstream:** [FR-202](./FR-202-formalization-coverage-state.md) defines item states; [FR-153](./FR-153-analysis-result-counting-by-disposition.md) is the only route to analysis-obligation credit; [IR-96](ix://agent-ix/quire-contract-ir/FR-006) owns the coverage-state record and clause-address contract.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-203-AC-1 | Given an item in state `approved`, `quoin assurance` shows one formalization evidence record on its requirement, conforming to IR-96, and no locally defined substitute record shape is emitted. | Test |
| FR-203-AC-2 | Given an item in state `analysed`, `quoin assurance` shows its formalization evidence with the corresponding IR-96 state for its QSL, lowering, and qualified-analysis progress. | Test |
| FR-203-AC-3 | Given an item in state `draft` or `none`, `quoin assurance` shows a gap for that item and emits no formalization evidence record for it. | Test |
| FR-203-AC-4 | Given a corpus and obligations with or without formalization evidence, every obligation has identical coverage credit in `quoin matrix`; formalization evidence alone grants no credit to Test, Analysis, or Inspection obligations. | Test |
| FR-203-AC-5 | Given a record that does not conform to the IR-96 coverage-state contract or clause address, Quoin rejects it and does not show it as formalization evidence. | Test |
