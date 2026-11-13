---
id: NFR-033
title: "Counting an analysis result never infers a binding and is order-independent"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-152"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-153"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
---

# NFR-033: Counting an analysis result never infers a binding and is order-independent

## Statement

Quoin SHALL create an obligation binding from an analysis run entry only when the recording command was given an
explicit link naming that obligation.

Quoin SHALL return the same counting result for the same store and the same supplied observations, whatever order the
run records are read or supplied in.

## Scope

- Applies to: `quoin evidence record` over a `differential-report` report (FR-152) and the counting of the resulting
  records through FR-078 and Quoin FR-116 (FR-153).
- Not applied to: the producer, and the other adapters, which keep the trace ids their producers state.

## Rationale

A binding inferred from a symbol or a digest can attach a result to the wrong requirement with nothing downstream
positioned to notice (ADR-100). A counting result that depends on read order lets a failed analysis be hidden behind
an older pass.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Bindings created from an analysis entry with no link, including a report whose symbol equals an obligation id | 0 | 0 | Test |
| Differing counting results across permutations of the supplied run records | 0 | 0 | Test |
| Obligations counted from a binding or a healthy audit without a FR-116 positive decision | 0 | 0 | Test |

## Verification

Covered by FR-152-AC-2, FR-152-AC-3, FR-153-AC-5 and FR-153-AC-8, executed as TC-002.
