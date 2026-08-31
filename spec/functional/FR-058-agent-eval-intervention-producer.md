---
id: FR-058
title: "First-party agent-evaluation intervention producer"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-015"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-056"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-057"
    type: "requires"
---

# FR-058: First-party agent-evaluation intervention producer

## Description

When Quoin receives baseline and treatment reports produced by real
`cli-agent-evals` runs and a governed producer definition declaring their
input-schema version, Quoin SHALL provide a first-party producer that converts
their observed scenario results into one FR-056 intervention-experiment record and
submits it through the FR-057 intake boundary.

## Inputs

- A producer-definition document naming the supported input-schema version,
  record/subject identity, the governing definition version, baseline and treatment
  configurations, changed and held variables, design and sampling conditions,
  declared interactions/confounders, owner, gaps, and actions.
- Exact baseline and treatment `cli-agent-evals` JSON reports retained from
  separately executed runs.
- Immutable Quoin and `cli-agent-evals` versions, source revisions, configuration
  digests, and environment identity.

## Outputs

- One accepted intervention-experiment record, or an actionable refusal from the
  existing FR-057 intake surface.
- Raw-evidence references to both unmodified input reports with computed media type,
  byte size, and content digest.

## Behavior

- The producer SHALL require the governed definition to declare the supported
  `cli-agent-evals` input-schema version before parsing either retained report.
  This declaration versions immutable historical output without editing it.
- The producer SHALL parse each report through Quoin's existing agent-eval adapter
  and SHALL refuse an empty, malformed, or structurally incompatible report.
- The producer SHALL require the baseline and treatment reports to name the same
  non-empty scenario-id set and SHALL refuse every missing, duplicate, or unmatched
  scenario id.
- For each scenario, the producer SHALL retain baseline and treatment sample counts
  and pass rates and SHALL compute the treatment-minus-baseline pass-rate effect
  without replacing an absent observation with zero.
- The producer SHALL derive the record observation timestamp from the treatment
  report's `generatedAt` clock; it SHALL NOT accept a caller-supplied replacement.
- The producer SHALL compute the raw-evidence metadata from the exact input bytes;
  it SHALL NOT accept a caller-supplied digest, size, or measured effect.
- The producer SHALL emit `cause_not_established` with `none` attribution confidence
  whenever either report lacks a valid repeated sample, any declared interaction or
  confounder is uncontrolled/unknown, or the definition does not supply a justified
  attribution method.
- The producer SHALL NOT infer `causal_effect_established`; that conclusion remains
  available to other producers that can satisfy FR-056's stronger design contract.
- The producer SHALL NOT invoke an agent, evaluation harness, experiment command, or
  other producer. It consumes already-retained reports and submits through FR-057.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-058-AC-1 | Two real agent-eval reports governed by a supported input-schema declaration and carrying the same non-empty scenario set produce treatment-linked observations whose sample counts, pass rates, computed effects, and observation timestamp match the reports. | Test (TC-1217) |
| FR-058-AC-2 | An absent or unsupported input-schema declaration and empty, malformed, structurally incompatible, duplicate-scenario, or baseline/treatment scenario-mismatch reports are refused without an intervention store entry. | Test (TC-1218) |
| FR-058-AC-3 | The observation timestamp and raw-evidence media type, byte size, and digest are computed from the exact unmodified reports; caller-supplied substitutes cannot change them. | Test (TC-1219) |
| FR-058-AC-4 | Inadequate repetition, an uncontrolled/unknown qualifier, or absent attribution method produces `cause_not_established` with `none` confidence and explicit gaps rather than a causal conclusion. | Test (TC-1220) |
| FR-058-AC-5 | A real two-run integration invokes `cli-agent-evals` outside Quoin, then the Quoin producer consumes the retained reports without spawning any process and persists the record through FR-057. | Test (TC-1221) |

## Constraints

- Real-run fixtures and transcripts SHALL satisfy the repository's content-rights
  policy; constructed reports SHALL NOT be labelled as real producer evidence.
- The producer is an adapter, not an experiment runner or causal-estimation engine.

## Dependencies

- [FR-056](./FR-056-intervention-experiment-record.md) defines the output contract.
- [FR-057](./FR-057-intervention-experiment-intake-report.md) owns validation,
  persistence, and reporting.
- [FR-042](./FR-042-agent-eval-evidence.md) defines the existing agent-eval parser
  and its refusal behavior.
