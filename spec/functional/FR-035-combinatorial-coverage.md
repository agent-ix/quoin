---
id: FR-035
title: "t-way coverage over a declared configuration space"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "extends"
  - target: "ix://agent-ix/quire-rs/FR-061"
    type: "requires"
---
# FR-035: t-way coverage over a declared configuration space

## Description

Config-space bugs hide in interactions no single-dimension test exercises, and *"we tested the
configurations"* is unquantifiable without a declared space.

`quire-rs` **FR-061** mints the obligation — *"2-way over these dimensions"* — and states the space
in the obligation's own statement. This computes what a run actually reached, and **names what it
did not**.

quoin computes; it neither declares nor generates. The dimensions come from the spec, the executed
configurations come from the consumer's CI, and producing the covering array is nobody's job here
(ADR-0011 invariant 1).

### The gap list is the deliverable

A percentage tells someone how much is missing. The list tells them **which combinations to run**,
which is the difference between a number and an action. The finding names the missing tuples, capped
at ten with the remainder counted, rather than reporting a ratio and leaving the reader to derive
the work.

### How a combinatorial obligation is recognised

`parseSpace(statement) !== null`. That **is** the test — there is no second flag to keep in
agreement with the first, and an obligation whose statement is ordinary prose is silently not one.

### A configuration a run exercised but the spec never declared covers nothing

Counting it would let a coverage number rise by testing something else. Only tuples the declared
space demands are counted, on both sides of the ratio.

### The advisor signal is structural, not prose

The existing `configuration-matrix` characteristic is a prose regex — `configuration`,
`feature flag`, `combination of`. A minted space reads
`2-way over features(default|python|wasm) target(linux|wasm32)` and matches **none** of them, so the
very obligations that most need the combinatorial method would be the ones it was never advised for.

A statement that parses as a configuration space is a configuration matrix **by construction**,
whatever words it contains. A structural test also cannot false-positive on prose, which a widened
regex would.

### Agreement with the engine is asserted, not assumed

`TC-183` computes the demanded-tuple count for the same space `quire-rs` `TC-925` computes it for,
and asserts the same number. If the two ever disagree, an obligation and its audit are describing
different spaces — and the obligation would be measured against a target that is not the one it
states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-035-AC-1 | The space — strength, dimensions, values — is parsed back out of the obligation statement. | Test (TC-180) |
| FR-035-AC-2 | A statement that is not a configuration space yields `null`, which is how a combinatorial obligation is told from every other kind. | Test (TC-181) |
| FR-035-AC-3 | Exclusions are read without being mistaken for dimensions. | Test (TC-182) |
| FR-035-AC-4 | The demanded-tuple count matches the number `quire-rs` computes for the same space. | Test (TC-183) |
| FR-035-AC-5 | A forbidden combination is not demanded. | Test (TC-184) |
| FR-035-AC-6 | The result names the combinations that never ran, not only how many. | Test (TC-185) |
| FR-035-AC-7 | A configuration exercising values the spec never declared covers nothing. | Test (TC-186) |
| FR-035-AC-8 | Full coverage reports an empty gap list. | Test (TC-187) |
| FR-035-AC-9 | `audit` reports `combinatorial-gap` naming the missing combinations. | Test (TC-188) |
| FR-035-AC-10 | An obligation whose demanded combinations all ran is healthy. | Test (TC-189) |
| FR-035-AC-11 | An obligation declaring no configuration space produces no combinatorial finding. | Test (TC-190) |
| FR-035-AC-12 | A declared space advises the combinatorial method structurally; ordinary prose does not. | Test (TC-191) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-035-CON-1 | quoin SHALL NOT generate or execute combinations (ADR-0011 invariant 1). The consumer's CI runs them. | Design | Inspection |
| FR-035-CON-2 | quoin SHALL NOT declare a configuration space. The dimensions are the spec's, read from the obligation quire mints. | Design | Inspection |
| FR-035-CON-3 | `combinatorial-gap` is `medium`. An incomplete covering array is ordinary work in progress; what would be severe is a run claiming a completeness it does not have, which the gap list makes impossible to do quietly. | Design | Test (TC-188) |

## Dependencies

- **Upstream**: `quire-rs` [FR-061](ix://agent-ix/quire-rs/FR-061) (mints the obligation and states the space), [FR-032](./FR-032-evidence-auditor.md)
- **Downstream**: a Generator emitting a covering-array skeleton — noted, not scoped
