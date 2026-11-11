---
id: SR-20261111185435
title: "Gap analysis of quoin PR 807 (FR-131 coverage)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@0daec931; FR-131 criteria, spec/matrix.md, spec/functional/FR-108-measurement-verdict-checker.md, rust/crates/quoin-measurement/tests/tc_2400_population_set.rs"
review_set: subset
---

# Gap analysis of quoin PR 807

## Summary

Ticket: PLAT-110. Planless. Plan completion: not assessed. Measured with quoin 0.30.0 and
quire 0.40.0 (engine 0.56.0), using `quoin matrix --json` and `quire matrix --scope .` on
0daec931.

FR-131-AC-1..AC-5 each have a tagged test (TC-2400..TC-2407), and each test asserts its
criterion's main path. No stub was found. The PR adds no FR-131 row to `spec/matrix.md`,
however, so every tag is reported as `criterion-tagged-outside-row` and `quoin matrix`
lists all five criteria as `static_status: untagged` with `binders: []`.

Collision check: neither origin/main (4eb93180) nor open PR #806 (TC-2300/2301) nor
PLAT-114 (FR-132, TC-2500+) uses FR-131 or TC-2400..TC-2407. #805 (FR-130,
TC-2200..2211) has merged. `git merge-tree` of the PR head against current main merges
`spec/functional/index.md` and `spec/spec.md` cleanly.

## Verdict

**FAIL (two medium, two low).** The matrix rows are missing, so FR-131 currently has zero
backing. The AC-3 adverse cases that fail in code (see the code-review SR) have no tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `spec/matrix.md` has no FR-131 coverage row and no TC-2400..TC-2407 rows. All 8 tests are reported `criterion-tagged-outside-row` ("expected a same-test tag for one of no declared matrix row"), and `quoin matrix` reports FR-131-AC-1..5 as `untagged` with no binders. Add the FR-131 row under Functional Requirement Coverage and the 8 TC rows under Test Case Summary | spec/matrix.md:236, spec/matrix.md:1379 |
| FND-002 | medium | The FR-131-AC-3 adverse cases have no tests: a non-object or null `population`, and a non-numeric stated count, under a set plan. Both are admitted today (code-review FND-001, FND-002). Add these cases to TC-2403/TC-2404 | rust/crates/quoin-measurement/tests/tc_2400_population_set.rs:240-282 |
| FND-003 | low | FR-108's reason table, the verdict vocabulary that FR-131 extends, does not list `population_outside_set` or `population_partial`. Its `population_malformed` row still describes only counts, not malformed id lists. TC-2407 does not exercise a `matched_ids` id outside the set, or a count disagreement, at verify | spec/functional/FR-108-measurement-verdict-checker.md:150-170 |
| FND-004 | low | The FR-131 title and rationale state unconditionally that percentage metrics are taken over a plan-enumerated population, and that a producer "cannot shrink" it. The behavior is opt-in: it applies only where a plan declares `population_set`. Scope the title and rationale the way the Description's "Where..." clause already does | spec/functional/FR-131-percentage-metrics-over-enumerated-population.md:3-30 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-131-AC-1, path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md, role: examined, excerpt: "A plan's statistical_design.population_set is read when present. ... refuses the plan load as QM-PLAN-INVALID"}
- {id: FR-131-AC-2, path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md, role: examined, excerpt: "intake ... admits a measured observation whose population.examined_ids is exactly the declared set"}
- {id: FR-131-AC-3, path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md, role: examined, excerpt: "stating no examined_ids or no matched_ids is refused as QM-POPULATION-UNSTATED ... disagrees with a stated count is refused as QM-POPULATION-MALFORMED"}
- {id: FR-131-AC-4, path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md, role: examined, excerpt: "outside the declared set is refused as QM-POPULATION-OUTSIDE-SET; ... omitting any declared id ... QM-POPULATION-PARTIAL"}
- {id: FR-131-AC-5, path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md, role: examined, excerpt: "quoin measurement verify applies FR-131-AC-3 and FR-131-AC-4 ..."}
```
