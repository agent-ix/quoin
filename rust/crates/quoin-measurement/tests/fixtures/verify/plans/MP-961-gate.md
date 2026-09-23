---
id: MP-961
title: Fixture gate on the pass rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: gate.pass_rate
definition_version: gate.pass-rate-v1
ground_truth_kind: mechanical
objective:
  direction: higher
statistical_design:
  population: every case in the fixture suite
  minimum_population: 10
  sampling: exhaustive
  repetitions: 1
  estimator: proportion
  error_model: none -- a deterministic suite
  uncertainty: none -- an exact proportion
  decision_rule:
    comparator: ge
    threshold: 0.8
protected_apparatus:
  - spec/assurance/MP-961-gate.md
negative_controls:
  - kind: apparatus-edit
    description: the plan document, which states the decision rule the verdict is graded by, is digested with every collection
relationships: []
---

# Fixture gate on the pass rate

## Comparison and Enforcement

Pass at a proportion of at least 0.8.
