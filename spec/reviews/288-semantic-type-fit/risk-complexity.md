---
id: SR-052
title: "Risk and complexity review of issue 288 semantic type-fit audit requirements"
type: SpecReview
analysis: risk-complexity
scope: "US-014, FR-051..FR-055, NFR-015..NFR-016"
review_set: all
---

# Risk and complexity review of issue 288 semantic type-fit audit requirements

## Summary

The work is high breadth but bounded technical risk because it is read-only. Complexity concentrates
in source identity, heterogeneous manifests, malformed Markdown, qualified duplicate types, and
keeping generated projections consistent with canonical records. The requirements convert each into
a finite record shape and reconciliation invariant.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-043 | low | Residual implementation risk is manageable with fixture-first parsers, pure assessment functions, canonical serialization, and a changed-path guard. | FR-051-AC-5..AC-6; FR-052-AC-7; FR-054-AC-5..AC-6; NFR-016 |

## Risk register

| Risk | Exposure | Control |
| --- | --- | --- |
| Mutable or mismatched source identities | high | Requested/resolved/inspected identities plus blocking conflict records |
| Corpus breadth creates silent omissions | high | Closed denominators and equality reconciliation |
| Heuristic scoring appears more certain than evidence | medium | Per-axis confidence/evidence and deferred disposition |
| Report and machine data diverge | medium | Generate both projections from one canonical object and digest the artifacts |
| Audit collides with active feature work | high | Read-only paths, stacked branch, fresh census, and major-interference gate |
