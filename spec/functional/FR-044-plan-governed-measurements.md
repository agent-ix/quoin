---
id: FR-044
title: "Plan-governed measurement store and report"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-043"
    type: "extends"
---

# FR-044: Plan-governed measurement store and report

## Description

Quoin SHALL persist one complete measurement producer invocation as one atomic
collection in the existing evidence store. Every observation SHALL resolve to
an active authored `MeasurementPlan` with the same definition version; an
unplanned measurement is refused rather than stored as an untyped number.

Quoin SHALL compare collections for compatibility before exposing a delta and
SHALL render the plans, current state, gaps, provenance, factual attention
items, comparisons, and series through `quoin report` without accepting a
hand-typed measurement value.

## Rationale

The Tier-1 benchmark previously appended a benchmark-specific JSONL envelope.
It preserved history but could not answer which plans governed the values,
could not serve other producers, and could not render current QA state. A bare
delta across changed definitions or configurations is confident misinformation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-044-AC-1 | A collection carries schema and collection identity, subject, scope, tool identity/version, configuration digest, timestamp, source revision, environment, observations with definition version and units, and attached raw evidence. Every observation resolves to an active plan or the write is refused naming the metric. | Test (TC-1003) |
| FR-044-AC-2 | A producer invocation lands by one same-directory atomic rename after validation. A partial or invalid invocation leaves no collection, and writing identical bytes for the same id is idempotent. | Test (TC-1004) |
| FR-044-AC-3 | Comparison refuses changed definitions, changed producer configuration, and incomplete populations with actionable reasons; surfaces tool and population movement; reports a missing metric as `not_computed`; and emits no quality verdict or severity. | Test (TC-1005..TC-1007) |
| FR-044-AC-4 | `quoin report` is a deterministic store view. It shows every active plan, a plan without a record as `not_computed`, all dimensional observations, corpus gap count, full producer provenance, and factual attention items. JSON, series, and since-revision views read the same store and accept no typed value. | Test (TC-1008) |
| FR-044-AC-5 | Tier-1 updates write generic collections first and only then refresh the derived ratchet baseline. Pre-plan JSONL observations remain explicitly legacy and are excluded from active reports and gates. | Test (TC-997, TC-998, TC-1000) |
| FR-044-AC-6 | When a collection is written, each `verificationStack.artifacts` name is resolved as a repository-relative path: artifact names must be safe relative paths, labels included. Every path component leading to the name is checked against the filesystem, left to right; a symlink at any component before the last refuses the write as `QM-ARTIFACT-UNREADABLE`, naming the artifact and the component. A name that is not a safe relative path refuses the write as `QM-ARTIFACT-NAME-UNSAFE`. Past those checks, a name that resolves to a regular file is digested, and a digest that disagrees with the submitted one refuses the write naming the artifact; a name that resolves to an entry that cannot itself be digested (a directory, a symlink, an unreadable or oversized file) also refuses it as `QM-ARTIFACT-UNREADABLE`. A name with no filesystem entry anywhere along its path is an artifact label and is admitted on its digest's shape. Both refusal codes name the artifact, and a refused write leaves no collection. | Test (TC-1731..TC-1733) |

## Constraints

- Comparability and policy are separate; this layer emits no merge verdict.
- A population change remains visible beside any numeric delta.
- Raw producer output stays attached and is never transcribed into authored Markdown.
- No second evidence store, QA artifact type, or finding vocabulary is introduced.

## Dependencies

- FR-030 defines the evidence-store integrity and atomic-write behavior reused here.
- FR-043 defines the finding-shaped analysis consumed by the report.
- The separately installed private engineering-assurance module defines
  `AssuranceProfile` and `MeasurementPlan`; Quoin's public default installer
  does not fetch it.
