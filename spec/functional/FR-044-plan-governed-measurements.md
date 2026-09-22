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
| FR-044-AC-6 | When a collection is written, each `verificationStack.artifacts` name is resolved as a repository-relative path: artifact names must be safe relative paths, labels included. Every path component leading to the name is checked against the filesystem, left to right; a symlink at any component before the last refuses the write as `QM-ARTIFACT-UNREADABLE`, naming the artifact and the component. A name that is not a safe relative path refuses the write as `QM-ARTIFACT-NAME-UNSAFE`. Past those checks, a name that resolves to a regular file is digested, and a digest that disagrees with the submitted one refuses the write naming the artifact; a name that resolves to an entry that cannot itself be digested (a directory, a symlink, an unreadable or oversized file) also refuses it as `QM-ARTIFACT-UNREADABLE`. A name with no filesystem entry anywhere along its path is an artifact label, and it stays admitted on its digest's shape — the owner's ruling is that quoin does not require every declared artifact to be locally reachable — but admission is never silent: every such name is recorded, sorted, as the collection's `verificationStack.unverifiedArtifacts`, a digested name never appears there, and a collection with nothing unverified states no such member at all. `quoin report`, in both its text and JSON views, states each unverified artifact beside the collection's provenance. Both refusal codes name the artifact, and a refused write leaves no collection. | Test (TC-1731..TC-1734) |
| FR-044-AC-7 | A `MeasurementPlan`'s `ground_truth_kind`, `statistical_design.minimum_population` and `statistical_design.repetitions` are read when present; a kind other than `human-labelled`, `agent-labelled` or `mechanical`, a count that is not a whole number from 1 to 4,294,967,295, or a `statistical_design` that is not an object refuses the plan load as `QM-PLAN-INVALID`, naming the member. When the governing plan declares `minimum_population`, intake of a new collection refuses each `measured` observation whose `population.examined` is below it as `QM-POPULATION-BELOW-MINIMUM`, and each `measured` observation stating no numeric `population.examined` as `QM-POPULATION-UNSTATED`; a population equal to the minimum is admitted. Each finding names the metric with its dimensions, the plan, and the minimum. Population findings accumulate with every other intake finding: the refusal carries the population code when every finding shares it, and `QM-COLLECTION-INVALID` otherwise, with each population finding leading with its own code. A plan stating none of these members loads and admits exactly as before. | Test (TC-1740..TC-1744, TC-1746) |
| FR-044-AC-8 | `quoin report` states a plan's `ground_truth_kind` beside the plan: the text view's and the portfolio view's Plan cell read `<id> (<path>; ground truth: <kind>)`, and the JSON views carry it as the plan's `groundTruthKind` member. A plan that states no kind renders the same bytes as before, with no such clause and no such member. | Test (TC-1744, TC-1745) |

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
