---
type: MeasurementPlan
id: ap-unmeasured
title: Never computed
status: active
stage: gate
metric: unmeasured_metric
definition_version: "1"
ground_truth_kind: mechanical
statistical_design:
  estimator: proportion
  decision_rule:
    comparator: ge
    threshold: 1.0
protected_apparatus:
  - spec/assurance/50-unmeasured.md
negative_controls:
  - kind: apparatus-edit
    description: the plan document
owner: qa
action: review
---

# Never computed
