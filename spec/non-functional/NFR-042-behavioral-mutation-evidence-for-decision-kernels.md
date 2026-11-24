---
id: NFR-042
title: "Behavioral mutation evidence for decision and binding kernels"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-039"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-160"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-184"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-208"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-211"
    type: "constrains"
---

# NFR-042: Behavioral mutation evidence for decision and binding kernels

## Statement

The local maintenance verification lane for the Quoin repository SHALL produce retained behavioral mutation results for the approval-decision, discharge, evidence-binding, risk-completeness and per-class-floor kernels.

## Scope

- Applies to: a maintainer's local verification of the Quoin Rust workspace and the referenced decision and evidence behaviors.
- Operational context: an explicitly invoked pre-release maintenance run. It governs verification of the repository's implementation, not the behavior of the distributed Quoin CLI.
- Not applied to: mutations that cannot compile or that do not reach a test run; those remain explicitly classified as unviable rather than counted as killed.

## Rationale

Compiling a mutated program does not show that a test distinguishes the behavior the requirement promises. A retained behavioral run makes killed and surviving mutations reviewable against the decision and binding requirements, including cases where a passing compile-only check would hide an ineffective test suite. The mutation runner is a repository-maintenance tool; Quoin's product boundary remains governed by FR-039-CON-1 and does not execute mutation runs.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Generated viable mutations with a recorded behavioral outcome | all | all | Test |
| Surviving mutations omitted from the retained result | 0 | 0 | Inspection |
| Surviving mutations without a recorded human review disposition | 0 | 0 | Inspection |

## Verification

The repository-maintenance lane changes the targeted implementation, runs the relevant tests against each generated mutation, and retains the raw result containing killed, surviving, and unviable outcomes. This raw run output is distinct from the Quoin evidence record, whose survivor projection remains governed by FR-039-AC-13. A compile-only check is not a behavioral mutation result. A mutation-run or test-execution error remains a failed run and is not converted into success. Each surviving mutation is reviewed by a person and its disposition is recorded; neither the lane nor Quoin assigns acceptance, risk, severity, or qualification from a mutation result.

## Dependencies

- **Upstream**: [FR-039](../functional/FR-039-mutation-score-threshold.md) defines the recorded mutation-score and survivor semantics; [FR-115](../functional/FR-115-evidence-backed-test-matrix.md) consumes mutation findings; [FR-160](../functional/FR-160-per-class-evidence-floors.md) defines configurable class floors; [FR-184](../functional/FR-184-analysis-act-producer-definition.md), [FR-208](../functional/FR-208-module-specific-discharge-approval.md), and [FR-211](../functional/FR-211-discharge-resolves-evidence-and-approvals.md) define approval and discharge behavior.
- **Downstream**: none.
