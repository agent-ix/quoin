---
id: FR-107
title: "Ratchet, target and gate stage verdicts in the measurement report"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-045"
    type: "extends"
---

# FR-107: Ratchet, target and gate stage verdicts in the measurement report

## Description

When a `MeasurementPlan` at the `ratchet` stage states an `objective`,
`quoin report` SHALL state whether the plan's newest value held against, or
regressed from, the best value any earlier collection measured for the same
plan, slice and `definition_version`. When a plan at the `target` stage states
an `objective` with a `bound`, `quoin report` SHALL state the newest value's
distance to that bound and whether it has been reached. When a plan at the
`gate` stage states an `objective`, `quoin report` SHALL state whether the
plan's `statistical_design.decision_rule` holds for the newest value — the
one stage verdict this requirement decides that is a real pass/fail result,
not progress information.

The `objective` block is engineering-assurance's (its FR-020): a `direction`
of `higher`, `lower`, `zero` or `target`, and an optional finite `bound` that
`target` requires. Quoin parses it into engineering-assurance's own type. A
`gate` plan's verdict never reads `objective.bound`: an objective's bound is
an informational goal everywhere in the measurement layer (owner ruling,
PLAT-956), and `decision_rule` is the only member a `gate` evaluates.

## Rationale

MeasurementPlans already name `ratchet`, `target` and `gate` stages, but
nothing applied them: `quoin report` showed the newest value beside the plan
and left the reader to remember the best earlier value, which way was better,
and whether a gate's rule held. A stage that is named and never applied reads
as enforced when it is not. `quoin measurement verify` (FR-108) already
decides a gate's pass/fail from `decision_rule` across a plan's whole,
order-attested history; this requirement puts the same evaluation — EA's own
`DecisionRule::holds`, no rule logic restated — into the report a reader
already looks at for every other stage, over the one row and its `earlier`
collections the report is already built from.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-107-AC-1 | A `MeasurementPlan`'s `objective` is read when present, as engineering-assurance's `Objective`. An `objective` that is not an object, states an unknown `direction` or an unknown member, states a `bound` that is not a finite number, states `direction: target` with no `bound`, or states `direction: zero` with a negative `bound` refuses the plan load as `QM-PLAN-INVALID`, naming `objective`. A `zero` bound of `0` is admitted. | Test (TC-1771) |
| FR-107-AC-2 | A value is usable evidence when its observation names the plan's id, is measured under the plan's `definition_version`, carries a value, and states a `population` that is neither incomplete nor empty. For a `ratchet` plan with an `objective`, the best earlier value is taken over the usable values of the same metric and dimensions in collections older than the one the report row quotes. Best is the maximum for `higher`, the minimum for `lower`, the value with the smallest magnitude for `zero`, and the value closest to the bound for `target`. The newest value is `held` when it is at least as good as the best earlier value and `regressed` otherwise; the report names the best earlier value and the collection that measured it, which is the earliest collection on a tie. | Test (TC-1760..TC-1763, TC-1766, TC-1772, TC-1773) |
| FR-107-AC-3 | A `ratchet` verdict is `inconclusive` with a reason code when the newest value is not usable evidence under AC-2: `no_current_value` (no measured value), `plan_mismatch` (it names another plan id), `definition_mismatch` (another `definition_version`), `population_unstated` (no `population`), `incomplete_population` or `empty_population` (`examined: 0`). It is also `inconclusive` with `no_prior` when no earlier usable value exists, which includes the first collection a ratchet sees. Each dimension slice with a usable earlier value that the newest collection does not measure at all is reported as its own `inconclusive` row with `no_current_value`. | Test (TC-1764..TC-1766, TC-1774..TC-1776) |
| FR-107-AC-4 | For a `target` plan with an `objective` and a `bound`, the report states whether the newest value has reached the bound and the distance still to go. `reached` is `current ≥ bound` for `higher`, `current ≤ bound` for `lower`, `\|current\| ≤ bound` for `zero`, and exact IEEE equality `current = bound`, with no tolerance, for `target`. The distance is `0` once reached; otherwise it is `bound − current`, `current − bound`, `\|current\| − bound` or `\|current − bound\|` respectively. This is progress information, not a pass/fail verdict. A newest value that is not usable evidence under AC-2 gives `inconclusive` with the AC-3 reason, and a `target` plan whose objective states no `bound` gives `inconclusive` with `no_bound`. Tested: `higher`, `lower` and `zero` both reached and not reached; `target` at exact equality and off it; no bound; incomplete and unstated populations. | Test (TC-1767, TC-1776, TC-1777) |
| FR-107-AC-5 | The text report prints a "Stage verdicts" table (Metric, Plan, Stage, Objective, Verdict, Detail) after the measured-plans table. A `regressed` ratchet and a dropped slice each add an attention item naming the slice and the plan. The JSON report carries the plan's `objective`, each row's `stageVerdict` and, when a slice was dropped, `vanishedSlices`. A ratchet's `stageVerdict` carries `verdict` (`held`, `regressed`, `inconclusive`); a target's carries `progress` (`reached`, `not_reached`, `inconclusive`). Each states every member of its stage, `null` where there is no such number. The portfolio text prints the same table under each repository, and the portfolio JSON carries the same members. | Test (TC-1769, TC-1770, TC-1775) |
| FR-107-AC-6 | Stage verdicts are reported only for plans that state an `objective` and sit at the `ratchet`, `target` or `gate` stage. For every other plan, the text, JSON and portfolio views are the same bytes they were before this requirement. | Test (TC-1768) |
| FR-107-AC-7 | For a `gate` plan with an `objective` and a `statistical_design.decision_rule`, the report evaluates the rule through engineering-assurance's `DecisionRule::holds` against the newest value: `pass` when it holds, `fail` otherwise. A `threshold` rule needs no baseline value. A `baseline` rule reads it from the same usable-evidence pool as AC-2, restricted to the plan's slice: `prior-collection` is the nearest earlier usable value, and `best-seen` is the maximum for `gt`/`ge` or the minimum for `lt`/`le`/`eq`. No `baseline` rule reads a value that is not usable evidence under AC-2, or a value of another slice. `objective.bound` is never read by a gate. | Test (TC-1868, TC-1869, TC-1873) |
| FR-107-AC-8 | A `gate` verdict is `inconclusive` with a reason code, never `pass`, when: the newest value is not usable evidence under AC-2 (the same reasons as AC-3); the plan states no `decision_rule` (`no_decision_rule`); a `baseline` rule finds no usable earlier value for the slice (`no_prior`, as a ratchet's first collection); the rule's baseline is `constant-predictor`, which needs per-item answers by answer family that no collection in the report layer carries (`constant_predictor_unsupported`); engineering-assurance could not evaluate the rule on the supplied numbers (`rule_not_evaluable`); or, for a `baseline` rule under a plan that protects apparatus, the ratchet's FR-110-AC-6 reasons hold (`apparatus_changed`, `apparatus_unrecorded`). | Test (TC-1870, TC-1871, TC-1873, TC-1874) |
| FR-107-AC-9 | The "Stage verdicts" table and the JSON `stageVerdict` state a gate the same way as a ratchet and a target: the table's `Verdict` column reads `pass`, `fail` or `inconclusive`, and its `Detail` states the current value and the baseline value when the rule used one. The JSON `stageVerdict` for a `gate` row carries `verdict` (`pass`, `fail`, `inconclusive`), `reason`, `current` and `baseline`, `null` where there is no such number. A `fail` gate adds an attention item naming the plan, alongside a `regressed` ratchet's. | Test (TC-1872) |

## Constraints

- **FR-107-CON-1**: Collection comparison (FR-044-AC-3) stays verdict-free;
  the verdict is decided in the report layer from the plan's own stage,
  objective and, for a gate, its decision rule.
- **FR-107-CON-2**: A `gate` verdict never reads `objective.bound`; the bound
  is informational everywhere in the measurement layer (owner ruling,
  PLAT-956) and `decision_rule` is the gate's only source of truth. A `gate`
  plan restates no rule-evaluation logic of its own: AC-7's evaluation is
  engineering-assurance's `DecisionRule::holds`, the same call
  `quoin measurement verify` (FR-108) makes.
- **FR-107-CON-3**: Under a plan that protects apparatus, FR-110-AC-6 adds
  two `inconclusive` reasons to a ratchet, `apparatus_changed` and
  `apparatus_unrecorded`, and the same two to a gate whose rule is a
  `baseline`, since a baseline is a floor measured by an earlier collection
  exactly as a ratchet's best is; plans that protect nothing are unaffected.
  A `threshold` gate compares against no earlier value and is not
  apparatus-checked here; FR-108 remains the full apparatus check over the
  plan's order-attested history.

## Dependencies

- FR-044 defines the plan-governed store and the report this extends.
- FR-045 defines the portfolio view this extends.
- engineering-assurance FR-020 owns the `objective` block's schema and type.
- engineering-assurance FR-021 owns `decision_rule`'s schema, type and
  evaluation (`DecisionRule::holds`), which AC-7 and AC-8 apply.
- FR-108 defines `quoin measurement verify`, the independent checker that
  evaluates the same rule over a plan's whole history; this requirement
  applies the same evaluation to one report row.
