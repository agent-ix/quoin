---
id: SR-20261111182230
title: "Gap analysis of quoin PR 806 (FR-108-AC-24/25)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@bf12d74d; spec/functional/FR-108-measurement-verdict-checker.md (AC-24, AC-25, reason table, constant-predictor limits), spec/matrix.md (FR-108 row, TC-2300, TC-2301), tagged tests"
review_set: subset
---

# Gap analysis of quoin PR 806

## Summary

Ticket: PLAT-1020. Planless, scoped to the PR diff. Plan completion: not assessed.
`quoin matrix --json` reports FR-108-AC-24 as tagged by `tc_1020_001` and FR-108-AC-25 as
tagged by `tc_1020_002`. Both use `#[trace("FR-108-AC-2x", "TC-230x")]`, and both are
undischarged because no evidence has been run, which is normal before merge. TC-2300 and
TC-2301 and FR-108-AC-24/25 do not occur on origin/main. The only other open PR, #805,
uses TC-2200..2211 and FR-130, so nothing collides. Its matrix edit sits in a different
region and may need a textual rebase only. `quire validate` on the two spec files reports
nothing new: the warning at line 196 (AC-21) and the TC-1943/1944 tombstone rows are
pre-existing. Spec-review was not run as a separate method because the brief scoped this
review to code-review, rust-review and gap-analysis. The AC wording was checked against
behaviour here instead.

## Verdict

**PASS WITH FINDINGS (two low).** AC-25 matches the behaviour exactly. AC-24 matches except
for the merge example, and the TC-2300 row cites an untagged unit test for the branches the
tagged test does not exercise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-24 (and the `constant_predictor_family_unexpected` reason-table row and TC-2300) give "two declared families merged into one" as the `family_unexpected` case. That holds only when the merged name is undeclared. A producer that writes every row under one declared name (for example `weakness_kind`) gets `constant_predictor_family_missing`, as the `Reason` doc in reason.rs correctly says. Reword the example to "merged under an undeclared name", or state both outcomes | spec/functional/FR-108-measurement-verdict-checker.md:150,199 |
| FND-002 | low | The TC-2300 matrix row cites `verify/constant_predictor.rs :: item_row_families_must_equal_the_declared_list` as backing, but that unit test has no `#[trace]` marker, so it binds nothing. The tagged `tc_1020_001` does not exercise AC-24's `families_unstated` branch or its "same families in any order are accepted" clause. Tag the unit test `#[trace("FR-108-AC-24", "TC-2300")]` or move those two assertions into the tagged test | spec/matrix.md:413, rust/crates/quoin-measurement/src/verify/constant_predictor.rs:484-520 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-108-AC-24, path: spec/functional/FR-108-measurement-verdict-checker.md, role: examined, excerpt: "A `constant-predictor` baseline holds the run's item rows' distinct `family` values against the plan's `statistical_design.answer_families`, compared as a set (engineering-assurance FR-021), before computing anything: a plan stating none is `inconclusive` with `constant_predictor_families_unstated`; an item row whose `family` is not declared (two declared families merged into one, or one split) is `constant_predictor_family_unexpected`; a declared family with no item row is `constant_predictor_family_missing`. No baseline is computed in any of these cases, the same families in any order are accepted, and an undeclared family is reported before a missing one."}
- {id: FR-108-AC-25, path: spec/functional/FR-108-measurement-verdict-checker.md, role: examined, excerpt: "A plan whose `statistical_design.decision_rule.baseline` is `constant-predictor` and which states no `statistical_design.answer_families` is refused at load as `QM-PLAN-INVALID` naming `statistical_design.answer_families`; a plan that states the list loads."}
- {id: TC-2300, path: spec/matrix.md, role: examined, excerpt: "Item rows whose families differ from the plan's `answer_families` (two declared families merged into one, one split, or a declared family absent) are `inconclusive` with `constant_predictor_family_unexpected` or `constant_predictor_family_missing`, a plan stating none with `constant_predictor_families_unstated`, with no baseline computed; the same set in any order is read"}
- {id: TC-2301, path: spec/matrix.md, role: examined, excerpt: "A plan declaring `baseline: constant-predictor` without `statistical_design.answer_families` is refused at load as `QM-PLAN-INVALID` naming the member; with the list it loads"}
- {id: FR-108 matrix row, path: spec/matrix.md, role: examined, excerpt: "PLAT-1020 holds the item rows' families to the plan's declared `statistical_design.answer_families` (FR-108-AC-24, AC-25): a merged, split, undeclared or missing family is `inconclusive` with its own reason, and a `constant-predictor` plan without the list is refused at load."}
- {id: FR-108 reason table, path: spec/functional/FR-108-measurement-verdict-checker.md, role: examined, excerpt: "| `constant_predictor_family_unexpected` | inconclusive | the run's constant-predictor item rows carry a `family` the plan's `answer_families` does not declare, such as two declared families written as one (PLAT-1020) |"}
- {id: tc_1020_001, path: rust/crates/quoin-measurement/tests/tc_1016_constant_predictor_intake.rs, role: examined, excerpt: "#[trace(\"FR-108-AC-24\", \"TC-2300\")] fn tc_1020_001_item_row_families_that_differ_from_the_declared_list_are_inconclusive()"}
- {id: tc_1020_002, path: rust/crates/quoin-measurement/tests/tc_1016_constant_predictor_intake.rs, role: examined, excerpt: "#[trace(\"FR-108-AC-25\", \"TC-2301\")] fn tc_1020_002_answer_families_are_required_with_constant_predictor()"}
```
