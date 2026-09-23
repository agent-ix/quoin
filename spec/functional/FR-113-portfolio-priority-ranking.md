---
id: FR-113
title: "Advisory portfolio priority ranking by weighted, time-discounted, budget-normalized gap to bound"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-045"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-107"
    type: "extends"
---

# FR-113: Advisory portfolio priority ranking by weighted, time-discounted, budget-normalized gap to bound

## Description

When engineering-assurance's PLAT-967 steering fields (`weight`, `value_half_life`,
`budget`) are declared on a `MeasurementPlan`'s `objective`, `quoin report --portfolio`
SHALL rank every plan whose `objective` states a `bound` and whose newest value is
usable evidence, by a score computed from that plan's gap to its bound, weighted,
discounted by staleness, and normalized by budget. A plan with no `objective`, an
`objective` with no `bound`, or no usable newest value SHALL be listed, with the
reason, and SHALL NOT be scored.

The ranking SHALL state, in both the rendered text and the rendered JSON, that it is
advisory: it decides nothing and gates nothing. It SHALL NOT feed
[`crate::report::verdict::stage_verdict`](../../rust/crates/quoin-measurement/src/report/verdict.rs)
or any `DecisionRule`, and no `gate` plan's pass/fail outcome (FR-107) SHALL depend on
it in either direction.

## Rationale

FR-045's portfolio view aggregates across repositories, profiles, plans, comparisons
and staleness, and makes no priority call — deliberately, per its own module doc: two
repositories' metrics are not commensurable, so summing or averaging them would be a
quality verdict this layer has no evidence for. That constraint still holds for a
metric *value*. It never applied to a plan's own declared priority: PLAT-967 gives a
plan's `objective` a `weight`, a `value_half_life` and a `budget`, each explicitly
advisory and structurally excluded from `DecisionRule::holds` and from the plan's
measurement definition. Nothing read them. A portfolio spanning many repositories'
plans is exactly where "what should someone attempt next" is worth asking, and it is
a question about priority among gaps, not about the gaps' values being commensurable —
so it can be answered without violating FR-045's own boundary.

## The formula

For a rankable plan with newest usable value `current`, objective `bound` and
`direction`, and optional `weight`, `value_half_life` and `budget`:

```text
gap           = shortfall(direction, current, bound)   // 0 once the bound is met
weight'       = weight.unwrap_or(1.0)
decay         = 0.5 ^ (age_days / value_half_life)   // 1.0 when value_half_life or age_days is absent
budget_floor  = max(budget.unwrap_or(1.0), 1.0)        // MIN_BUDGET_FLOOR = 1.0
score         = (weight' * gap * decay) / budget_floor
```

`age_days` is the newest value's collection timestamp's distance, in whole days, from
the portfolio's own newest collection timestamp — the same reference point FR-045's
`Staleness` already uses, so "how old" means one thing across the whole view. Ranked
descending by `score`, ties broken by repository name then plan id.

**Weighted gap-to-bound.** `gap` is how far the value still falls short of the
`bound`, read in the objective's `direction`: `higher` → `max(bound - current, 0)`,
`lower` → `max(current - bound, 0)`, `zero` → `max(|current| - |bound|, 0)`, and
`target` → `|current - bound|`, the distance FR-107's `target` progress already
computes (`TargetOutcome::Measured::distance`), since overshooting a target misses it
too. A `higher`/`lower`/`zero` objective may state a `bound` even though only `target`
requires one, and the ranking needs some goal to measure distance from. A plan that
has met or passed its bound has a gap of `0`, scores `0` and sorts last; a
direction-blind `|current - bound|` would instead rank a `higher` plan that cleared
its bound by a wide margin as the most urgent. `weight` scales the
gap linearly: EA's own doc states it as "relative value against the project's other
objectives", and doubling the weight doubles the plan's claim on attention for the
same gap.

**Value half-life discount.** An old, unaddressed gap must not dominate the ranking
forever just because nobody has re-measured it. `decay` halves the gap's contribution
every `value_half_life` days of staleness: full strength at `age_days = 0`, half at
`age_days = value_half_life`, and shrinking without ever reaching exactly zero — an
old gap counts for less, it is not silently erased, so a fresh, small gap in a
different plan can still outrank it. No `value_half_life`, or an unparsable or missing
collection timestamp, applies no discount (`decay = 1.0`) rather than guessing one.

**Budget normalization.** `budget` is "the time, token, or compute budget per
attempt" (EA's doc), not a total remaining-room figure, so dividing by it reads as
*gap per unit of what an attempt costs* — the "bang for the buck" a portfolio-wide
ranking needs. Dividing by a `budget` near zero would send the score toward infinity
for no real reason, so a floor of `1.0` applies before the division; no stated
`budget` uses the same floor, so an unstated budget and a budget the floor exceeds
are ranked identically.

These choices are a design decision, not something PLAT-968's acceptance criteria
mandate — the ticket asked for judgment on the exact decay function and left the
rest to design. The full reasoning, including why each steering field defaults to a
neutral value rather than excluding the plan when absent, is recorded in
`rust/crates/quoin-measurement/src/portfolio/ranking.rs`'s module doc, which this
requirement's tests trace against directly.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-113-AC-1 | A plan whose `objective` states a `bound` and whose newest value is usable evidence (as FR-107-AC-2 defines usable) is scored by `weight' * gap * decay / budget_floor` and ranked descending by score. A declared `weight` changes the order relative to the unweighted default of `1.0` for an otherwise smaller gap. `gap` is read in the objective's direction, so a met or passed bound has a gap of `0` (`target` excepted: overshoot is a gap). | Test (TC-1904, TC-1905, TC-1914) |
| FR-113-AC-2 | `decay` halves the score's gap contribution every `value_half_life` days the newest value's collection is behind the portfolio's newest collection timestamp, computed as `0.5 ^ (age_days / value_half_life)`; a stale, large gap can rank behind a fresh, smaller one. No `value_half_life` applies no discount regardless of age. | Test (TC-1906, TC-1907) |
| FR-113-AC-3 | The score is divided by `max(budget, 1.0)` when a `budget` is stated, and by `1.0` when it is not; a larger budget lowers the score for the same gap, and a budget below the floor is ranked identically to no budget stated. | Test (TC-1908) |
| FR-113-AC-4 | A plan with no `objective`, an `objective` with no `bound`, or no usable newest value is listed in the ranking's unranked set with the reason (`no_objective`, `no_bound`, `no_current_estimate`) and never appears in the ranked, scored list. | Test (TC-1909, TC-1910, TC-1911) |
| FR-113-AC-5 | The rendered text carries a "Priority ranking" section and the rendered JSON carries a `ranking` member, both present unconditionally (even when nothing is ranked); both state verbatim that the ranking is advisory, decides nothing and is not a gate. | Test (TC-1912, TC-1913) |

## Constraints

- **FR-113-CON-1**: The ranking reads a plan's `objective` and the report's already-
  computed rows; it runs no producer, writes no store, and evaluates no
  `DecisionRule`. Removing this module removes a sort order, not a fact any other
  view depends on.
- **FR-113-CON-2**: "Usable evidence" is the one rule FR-107-AC-2 already states for
  a stage verdict's newest value; this requirement does not restate it. A row's newest
  value must be usable to be scored, exactly as it must be for a ratchet or target
  verdict.
- **FR-113-CON-3**: The ranking never changes a `gate` plan's `pass`/`fail` outcome,
  a `ratchet`'s `held`/`regressed` outcome, or `objective.bound`'s status as
  informational-only (the epic-wide ruling on PLAT-956, restated by FR-107-CON-2).

## Dependencies

- FR-045 defines the portfolio view this extends, and the boundary ("no cross-
  repository aggregate, no quality verdict") this requirement's advisory ranking
  stays inside.
- FR-107 defines the stage-verdict machinery this reuses "usable evidence" from, and
  the `target` progress distance this requirement's `gap` generalizes.
- engineering-assurance FR-020 owns `objective`'s `direction` and `bound`; FR-026
  (PLAT-967) owns the steering fields `weight`, `value_half_life` and `budget` this
  requirement reads, declared advisory and structurally excluded from
  `DecisionRule::holds` and from the plan's measurement definition.
