---
id: FR-045
title: "Complete measurement intake and code-health observation adapters"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-043"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-044"
    type: "requires"
---

# FR-045: Complete measurement intake and code-health observation adapters

## Description

`quoin evidence measure` SHALL transcribe an already-produced observation report into
one generic measurement collection. It SHALL accept a normalized open envelope and pure
readers for selected structural, churn, and clone-pair formats. It SHALL run no producer,
supply no threshold, and derive no aggregate code-health verdict.

Quoin SHALL validate every batch completely before its first write. When input has a
producer-reported limitation, zero observations, an expected-population mismatch,
duplicate stable subject, outside-repository path, non-finite value, or invalid record,
Quoin SHALL stop before storing the batch. The raw input receives one content digest
shared by the batch; the caller supplies authored plan/definition identity and collection
provenance.

Language-appropriate adapters retain the producer's own definition. Values from
different tools are not normalized merely because each calls a measure complexity.
Fallback structural intake excludes implementation/class aggregates and anonymous nodes,
then emits one stable source-file observation with the function-value distribution. It
does not manufacture durable per-function identity from lossy names or line numbers.
Churn and age remain separate observations. Clone intake
uses pair plus fragment identity rather than a repository percentage grade.

Quoin SHALL transcribe dependency-cruiser JSON as finding-shaped scan evidence because it
reports project-owned rule violations. When a traversal covers zero modules or evaluates
zero rules, Quoin SHALL refuse to record the result as a clean scan.

## Inputs

- A completed raw producer output or normalized observation envelope
- Authored measurement-plan and definition version
- Logical repository and exact source revision
- Producer, configuration, environment, collection time, and optional sampling identity
- Optional exact expected observation count and raw-evidence reference

## Outputs

- One canonical collection containing logical `MeasurementRecord` observations
- One atomic command outcome naming the stored collection path
- Finding-shaped dependency-rule scan records through `evidence record`

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-045-AC-1 | Fallback structural adapters exclude aggregate/anonymous nodes, deduplicate repeated analyzer projections, and emit stable per-file distributions rather than false per-function identities. | Test (TC-290) |
| FR-045-AC-2 | Current-file churn and line age are emitted as separate observations and are never multiplied into a score. | Test (TC-291) |
| FR-045-AC-3 | Clone reports emit one duplicated-line observation per path-pair and fragment identity rather than a repository grade. | Test (TC-292) |
| FR-045-AC-4 | Producer limitations, duplicate subjects, and outside-repository paths are rejected or carried as incomplete input for the command to reject. | Test (TC-293) |
| FR-045-AC-5 | dependency-cruiser rule edges become finding-shaped records with ruleset identity; zero traversed modules and zero evaluated rules are refused as vacuous. | Test (TC-294) |
| FR-045-AC-6 | `evidence measure` validates an exact expected population and every observation before atomically writing one canonical collection with raw-input digest and caller provenance. | Test (TC-295) |
| FR-045-AC-7 | The normalized envelope remains an open extension seam for measures and producers unknown to Quoin. | Test (TC-296) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-045-CON-1 | Quoin SHALL transcribe raw output without executing an analyzer or version-control mining tool. | Design | Inspection |
| FR-045-CON-2 | No adapter SHALL add a threshold, severity, repository grade, cross-tool normalization, or compliance claim. | Design | Test (TC-290..TC-296) |
| FR-045-CON-3 | Raw-output adapters SHALL be pure over supplied text and path-normalization context. | Design | Inspection |
| FR-045-CON-4 | A failed batch SHALL leave no partial measurement population in the evidence store. | Design | Test (TC-295) |

## Dependencies

- **Upstream**: [FR-043](./FR-043-generic-measurement-record.md), [FR-044](./FR-044-generic-measurement-comparisons.md)
- **Downstream**: coding-agent review context, project-owned code-health profiles, guided measurement promotion
