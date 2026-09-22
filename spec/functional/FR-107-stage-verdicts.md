---
id: FR-107
title: "Ratchet and target stage verdicts in the measurement report"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-045"
    type: "extends"
---

# FR-107: Ratchet and target stage verdicts in the measurement report

## Description

When a `MeasurementPlan` at the `ratchet` stage states an `objective`,
`quoin report` SHALL state whether the plan's newest value held against, or
regressed from, the best value any earlier collection measured for the same
plan, slice and `definition_version`. When a plan at the `target` stage states
an `objective` with a `bound`, `quoin report` SHALL state the newest value's
distance to that bound and whether it has been reached.

The `objective` block is engineering-assurance's (its FR-020): a `direction`
of `higher`, `lower`, `zero` or `target`, and an optional finite `bound` that
`target` requires. Quoin parses it into engineering-assurance's own type.

## Rationale

MeasurementPlans already name `ratchet` and `target` stages, but nothing
applied them: `quoin report` showed the newest value beside the plan and left
the reader to remember the best earlier value and which way was better. A
stage that is named and never applied reads as enforced when it is not.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-107-AC-1 | A `MeasurementPlan`'s `objective` is read when present, as engineering-assurance's `Objective`. An `objective` that is not an object, states an unknown `direction` or an unknown member, states a `bound` that is not a finite number, or states `direction: target` with no `bound` refuses the plan load as `QM-PLAN-INVALID`, naming `objective`. | Test (TC-1771) |
| FR-107-AC-2 | For a `ratchet` plan with an `objective`, the best earlier value is taken over collections older than the one the report row quotes, counting only observations of the same plan, metric and dimensions, measured under the plan's `definition_version`, with a value, and with a population that is neither incomplete nor empty. Best is the maximum for `higher`, the minimum for `lower`, the value with the smallest magnitude for `zero`, and the value closest to the bound for `target`. The newest value is `held` when it is at least as good as the best earlier value and `regressed` otherwise; the report names the best earlier value and the collection that measured it, the earliest on a tie. | Test (TC-1760..TC-1763, TC-1766) |
| FR-107-AC-3 | A `ratchet` verdict is `inconclusive`, never `held`, with a reason code, when: the newest collection has no measured value for the row (`no_current_value`); the newest value was measured under another `definition_version` (`definition_mismatch`); its population is incomplete (`incomplete_population`) or examined nothing (`empty_population`); no earlier collection qualifies under AC-2, which includes the first collection a ratchet ever sees (`no_prior`); or the objective needs a bound it does not state (`no_bound`). | Test (TC-1764..TC-1766) |
| FR-107-AC-4 | For a `target` plan with an `objective`, the report states the newest value's distance to the bound — `\|current − bound\|`, or `\|\|current\| − \|bound\|\|` for `zero` — and whether it is reached: `current ≥ bound` for `higher`, `current ≤ bound` for `lower`, `current = bound` for `target`, `\|current\| ≤ \|bound\|` for `zero`. This is information, not a pass/fail verdict. The newest-value conditions of AC-3 and a missing bound give `inconclusive` with the same reason codes. | Test (TC-1767) |
| FR-107-AC-5 | The text report prints a "Stage verdicts" table (Metric, Plan, Stage, Objective, Verdict, Detail) after the measured-plans table, and a `regressed` ratchet adds an attention item naming the newest value, the best earlier value, its collection and the plan. The JSON report carries the plan's `objective` and the row's `stageVerdict`, stating every member of its stage — `null` where there is no such number. The portfolio text prints the same table under each repository, and the portfolio JSON carries the same members. | Test (TC-1769, TC-1770) |
| FR-107-AC-6 | A plan with no `objective`, or one at any stage other than `ratchet` or `target`, carries no verdict: the text report prints no "Stage verdicts" table for it and no attention item, and the JSON states no `stageVerdict` for its rows and no `objective` for the plan. | Test (TC-1768) |

## Constraints

- **FR-107-CON-1**: Collection comparison (FR-044-AC-3) stays verdict-free;
  the verdict is decided in the report layer from the plan's own stage and
  objective.
- **FR-107-CON-2**: The `gate` stage is not decided by this requirement.

## Dependencies

- FR-044 defines the plan-governed store and the report this extends.
- FR-045 defines the portfolio view this extends.
- engineering-assurance FR-020 owns the `objective` block's schema and type.
