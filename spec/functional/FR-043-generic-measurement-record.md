---
id: FR-043
title: "Generic measurement records with definition and provenance identity"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
---

# FR-043: Generic measurement records with definition and provenance identity

## Description

`quoin` SHALL persist policy-free numeric observations as a distinct,
schema-versioned `MeasurementRecord`. Each record SHALL identify the authored
measurement plan and measure-definition version, observed subject and scope,
source revision, value and unit, collection tool/configuration, relevant
environment and sampling identity, collection time, and raw evidence.

The record exists for observations whose primary meaning is comparison over
time or between revisions: command latency, function complexity, dependency
count, resource use, and measures not yet invented. It carries no threshold,
verdict, named external standard, or closed measure vocabulary. Those are
authored policy and belong to the comparison layer, not the observation.

### Record selection decision table

| Evidence fact | Record | Required identity | Meaning deliberately absent |
|---------------|--------|-------------------|------------------------------|
| A suite executed and reported outcomes for stable test symbols | `RunRecord` | suite, commit, symbol | A generic project/code observation |
| A scanner executed and reported zero or more potential violations | `FindingRecord` | suite, commit, rule id/ruleset | A pass/fail verdict inferred from finding count |
| A tool observed a numeric property for a non-test subject, for later comparison | `MeasurementRecord` | plan + definition version, subject, scope, revision | Threshold, gate, aggregate quality score |

A metric-bearing `RunEntry` remains correct when the number is part of a test
execution, such as a mutation score for a declared mutation suite. The same
number is not copied into `MeasurementRecord` merely because it is numeric.

### Identity and persistence

Records live under `spec/evidence/measurements/<plan-id>/`. The filename is a
digest of definition version, subject, scope, and source revision. The digest
keeps arbitrary subject identities out of filesystem paths and makes the same
observation identity last-write-wins without reading the clock.

The primary `value` is always present. A distribution is optional and explicit:
sample count, summary values, and `(probability, value)` quantile pairs retain
their mathematical meaning without encoding it in names such as `p95_ms`.

## Inputs

- A machine observation already produced by the caller's tool
- An authored measurement-plan id and versioned measure definition
- Caller-supplied source, environment, sampling, tool, and raw-evidence identity

## Outputs

- `spec/evidence/measurements/<plan-id>/<identity-digest>.json`
- `measurement-record-v1.schema.json`, the authoritative record contract

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-043-AC-1 | A CLI-latency distribution and a per-function complexity observation both satisfy the same open, measure-neutral schema. | Test (TC-277) |
| FR-043-AC-2 | Missing unit, missing plan/definition identity, and empty subject or scope identity are rejected before a file is written. | Test (TC-278) |
| FR-043-AC-3 | Every non-finite primary, distribution, or quantile value is rejected before JSON serialization can turn it into `null`. | Test (TC-279) |
| FR-043-AC-4 | The same record writes byte-identically and resolves to the same schema-versioned path; a changed subject, scope, revision, or definition version changes that path. | Test (TC-280) |
| FR-043-AC-5 | Tool version, configuration digest, environment/sampling identity, collection timestamp, and raw-evidence digest/reference survive a write/read round trip. | Test (TC-281) |
| FR-043-AC-6 | The JSON Schema is closed to undeclared fields and contains no external-standard or hard-coded measure name. | Test (TC-282) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-043-CON-1 | A measurement record SHALL NOT contain a threshold, verdict, gate, aggregate score, or compliance claim. | Design | Test (TC-282) |
| FR-043-CON-2 | quoin SHALL NOT execute the measurement tool; it transcribes an observation the caller produced. | Design | Inspection |
| FR-043-CON-3 | A numeric RunEntry that belongs to a test execution SHALL remain a RunEntry rather than being duplicated as a measurement. | Design | Inspection |
| FR-043-CON-4 | Field names and schema enums SHALL NOT name an external standard or hard-code a measure. | Design | Test (TC-282) |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) (canonical file-store conventions)
- **Downstream**: authored measurement plans, comparison policy, regression gates, and reporting views
