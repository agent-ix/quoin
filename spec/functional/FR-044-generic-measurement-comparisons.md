---
id: FR-044
title: "Generic measurement queries, comparisons, trends, and ratchets"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-043"
    type: "requires"
---

# FR-044: Generic measurement queries, comparisons, trends, and ratchets

## Description

`quoin` SHALL query `MeasurementRecord` observations by authored plan,
definition version, subject, scope, and source revision. It SHALL compare
baseline and candidate populations only after establishing compatible identity,
and SHALL expose deterministic base/branch deltas, time-ordered trends, and an
optional ratchet whose direction and tolerance come entirely from caller policy.

The engine knows arithmetic and identity. It does not know measure ids, analyzer
names, preferred directions, or acceptable values. That separation keeps a new
measure a data/profile addition rather than a Quoin code change.

### Comparison state model

Compatibility is checked before arithmetic and before policy:

| State | Meaning | Ratchet applied |
|-------|---------|-----------------|
| `missing-baseline` | Candidate observations exist, but no baseline was supplied. | No |
| `empty-population` | The candidate population is empty (including both sides empty). | No |
| `partial-collection` | Baseline and candidate subject/scope populations differ, or the same sampling design reports different collected sample counts. | No |
| `incompatible` | A population is ambiguous, or paired records differ in plan, definition, unit, environment, or sampling identity. | No |
| `comparable` | Both complete populations carry compatible identities. | Only when caller policy was supplied |

These states are machine-distinct. Missing, partial, changed-definition, and
zero-delta are not alternate spellings of "clean".

### Generic policy

An optional policy supplies exactly two facts:

- whether an increase or a decrease is adverse; and
- a finite, non-negative absolute or relative tolerance.

Without policy, Quoin reports deltas and no verdict. With policy, it applies the
ratchet only after a compatible baseline is established and reports whether it
was actually applied. A relative policy against a zero baseline is an explicit
non-comparison because the ratio is undefined.

### Trend continuity

A trend covers one subject/scope population and sorts by collection timestamp,
source revision, and stable population key. Changes to plan, definition, unit,
environment, or sampling identity start a visible discontinuity rather than
joining unlike values into one continuous series.

## Inputs

- Baseline and candidate `MeasurementRecord` populations
- Optional query identity
- Optional caller-authored comparison policy

## Outputs

- Exact-query results in deterministic order
- Machine-readable comparison states and deltas
- Deterministic canonical JSON and review-oriented Markdown
- Time-ordered trend points with continuity/discontinuity markers

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-044-AC-1 | Stored records are queryable by plan, definition version, subject, scope, and source revision in deterministic order. | Test (TC-283) |
| FR-044-AC-2 | One comparison path produces deltas for both CLI-latency and per-function-complexity records without branching on measure or tool name. | Test (TC-284) |
| FR-044-AC-3 | Changed definition, unit, environment, or sampling identity produces an explicit incompatibility and no arithmetic result. | Test (TC-285) |
| FR-044-AC-4 | Missing baseline, empty population, missing subjects, sampling-count mismatch, mixed revision, and duplicate subject are distinct machine states/reasons. | Test (TC-286) |
| FR-044-AC-5 | A ratchet applies only when compatible baseline and caller policy both exist; invalid tolerance and a relative policy over zero are refused explicitly. | Test (TC-287) |
| FR-044-AC-6 | Trends are time ordered and mark definition/identity changes as discontinuities. | Test (TC-288) |
| FR-044-AC-7 | JSON and Markdown outputs are deterministic and engine source contains no known measure id, analyzer name, or universal threshold. | Test (TC-289) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-044-CON-1 | Quoin SHALL NOT supply a default direction, tolerance, or universal threshold. | Design | Test (TC-287, TC-289) |
| FR-044-CON-2 | Quoin SHALL NOT normalize values across units, definition versions, environments, or populations. | Design | Test (TC-285, TC-286) |
| FR-044-CON-3 | Comparison results SHALL remain observations/policy results, not `FindingRecord` scanner evidence. | Design | Inspection |
| FR-044-CON-4 | Query and render ordering SHALL use locale-independent comparisons. | Design | Test (TC-283, TC-289) |

## Dependencies

- **Upstream**: [FR-043](./FR-043-generic-measurement-record.md)
- **Downstream**: bundle-scale performance profiles, code-health profiles, CI gates, and reporting views
