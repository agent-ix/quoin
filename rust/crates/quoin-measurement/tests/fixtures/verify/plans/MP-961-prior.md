---
id: MP-961-prior
title: Fixture ratchet on the prior collection
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: gate.prior_rate
definition_version: gate.prior-rate-v1
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
    baseline: prior-collection
protected_apparatus:
  - spec/assurance/MP-961-prior.md
negative_controls:
  - kind: apparatus-edit
    description: the plan document, which states the decision rule the verdict is graded by, is digested with every collection
relationships: []
---

# Fixture ratchet on the prior collection

## Comparison and Enforcement

Pass when the proportion is no lower than the prior collection.
