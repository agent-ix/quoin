---
id: StR-007
title: "Assurance owners inspect governed evidence without rerunning producers"
type: StR
verification_method: test
evidence:
  - kind: test_case
    ref: tests/graph-adapters.test.ts
  - kind: test_case
    ref: tests/graph-portfolio.test.ts
relationships:
  - target: "ix://agent-ix/quoin/FR-066"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-067"
    type: "satisfied_by"
---
# StR-007: Assurance owners inspect governed evidence without rerunning producers

## Stakeholder Need

Assurance owners require that Quoin shall accept retained, identity-complete
graph evidence and render compatible cross-repository views without invoking a
producer, collapsing unavailable inputs into zero, or combining unlike
populations, so that review remains reproducible and does not turn evidence
transport into a new measurement authority.

## Rationale

Graph exports and graph-quality observations are produced in other bounded
systems. Re-running those systems while rendering a portfolio would make a
read-only review depend on ambient tools and mutable repositories. Lossless
intake plus premise-preserving reporting keeps the evidence reviewable while
leaving production and judgment with their declared owners.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-007-VC-1 | Versioned adapters retain complete producer identity and raw evidence, and portfolio reports preserve partitions, availability, and compatibility premises without executing a producer or deriving an aggregate verdict. | Integration (TC-1316) |

## Stakeholders

The primary stakeholders are assurance owners reviewing several repositories.
Affected parties are Quire and graph-quality producer maintainers whose output
contracts are consumed but not reinterpreted by Quoin.

## Dependencies

FR-066 owns lossless intake and FR-067 owns read-only portfolio reporting.
