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
| FR-107-AC-1 | A `MeasurementPlan`'s `objective` is read when present, as engineering-assurance's `Objective`. An `objective` that is not an object, states an unknown `direction` or an unknown member, states a `bound` that is not a finite number, states `direction: target` with no `bound`, or states `direction: zero` with a negative `bound` refuses the plan load as `QM-PLAN-INVALID`, naming `objective`. A `zero` bound of `0` is admitted. | Test (TC-1771) |
| FR-107-AC-2 | A value is usable evidence when its observation names the plan's id, is measured under the plan's `definition_version`, carries a value, and states a `population` that is neither incomplete nor empty. For a `ratchet` plan with an `objective`, the best earlier value is taken over the usable values of the same metric and dimensions in collections older than the one the report row quotes. Best is the maximum for `higher`, the minimum for `lower`, the value with the smallest magnitude for `zero`, and the value closest to the bound for `target`. The newest value is `held` when it is at least as good as the best earlier value and `regressed` otherwise; the report names the best earlier value and the collection that measured it, which is the earliest collection on a tie. | Test (TC-1760..TC-1763, TC-1766, TC-1772, TC-1773) |
| FR-107-AC-3 | A `ratchet` verdict is `inconclusive` with a reason code when the newest value is not usable evidence under AC-2: `no_current_value` (no measured value), `plan_mismatch` (it names another plan id), `definition_mismatch` (another `definition_version`), `population_unstated` (no `population`), `incomplete_population` or `empty_population` (`examined: 0`). It is also `inconclusive` with `no_prior` when no earlier usable value exists, which includes the first collection a ratchet sees. Each dimension slice with a usable earlier value that the newest collection does not measure at all is reported as its own `inconclusive` row with `no_current_value`. | Test (TC-1764..TC-1766, TC-1774..TC-1776) |
| FR-107-AC-4 | For a `target` plan with an `objective` and a `bound`, the report states whether the newest value has reached the bound and the distance still to go. `reached` is `current ≥ bound` for `higher`, `current ≤ bound` for `lower`, `\|current\| ≤ bound` for `zero`, and exact IEEE equality `current = bound`, with no tolerance, for `target`. The distance is `0` once reached; otherwise it is `bound − current`, `current − bound`, `\|current\| − bound` or `\|current − bound\|` respectively. This is progress information, not a pass/fail verdict. A newest value that is not usable evidence under AC-2 gives `inconclusive` with the AC-3 reason, and a `target` plan whose objective states no `bound` gives `inconclusive` with `no_bound`. Tested: `higher`, `lower` and `zero` both reached and not reached; `target` at exact equality and off it; no bound; incomplete and unstated populations. | Test (TC-1767, TC-1776, TC-1777) |
| FR-107-AC-5 | The text report prints a "Stage verdicts" table (Metric, Plan, Stage, Objective, Verdict, Detail) after the measured-plans table. A `regressed` ratchet and a dropped slice each add an attention item naming the slice and the plan. The JSON report carries the plan's `objective`, each row's `stageVerdict` and, when a slice was dropped, `vanishedSlices`. A ratchet's `stageVerdict` carries `verdict` (`held`, `regressed`, `inconclusive`); a target's carries `progress` (`reached`, `not_reached`, `inconclusive`). Each states every member of its stage, `null` where there is no such number. The portfolio text prints the same table under each repository, and the portfolio JSON carries the same members. | Test (TC-1769, TC-1770, TC-1775) |
| FR-107-AC-6 | Stage verdicts are reported only for plans that state an `objective` and sit at the `ratchet` or `target` stage. For every other plan, the text, JSON and portfolio views are the same bytes they were before this requirement. | Test (TC-1768) |

## Constraints

- **FR-107-CON-1**: Collection comparison (FR-044-AC-3) stays verdict-free;
  the verdict is decided in the report layer from the plan's own stage and
  objective.
- **FR-107-CON-2**: The `gate` stage is not decided by this requirement.

## Dependencies

- FR-044 defines the plan-governed store and the report this extends.
- FR-045 defines the portfolio view this extends.
- engineering-assurance FR-020 owns the `objective` block's schema and type.
