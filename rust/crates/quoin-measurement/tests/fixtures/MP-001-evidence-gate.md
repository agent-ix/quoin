---
id: MP-001
title: Contract analysis measurement plan
type: MeasurementPlan
status: proposed
owner: analyze-maintainers
metric: analysis_semantic_fault_differential_and_evidence_conformance
definition_version: quire-analyze.measurement-v1
stage: gate
ground_truth_kind: mechanical
statistical_design:
  population: declared test procedures
  minimum_population: 12
  sampling: each declared procedure once per repetition
  repetitions: 3
  estimator: count
  error_model: count adverse outcomes
  uncertainty: unavailable and inconclusive outcomes remain visible
  decision_rule:
    comparator: eq
    threshold: 0
negative_controls:
  - kind: suppressed-observation
    description: Every declared procedure has a retained outcome.
  - kind: selective-reporting
    description: Every repetition and solver outcome is retained.
relationships:
  - target: ix://agent-ix/quire-analyze/AP-001
    type: measures
---
# Contract analysis measurement plan

Measurements inform a later human source-release decision.
