---
id: SR-20261103118172
title: "Gap analysis of quoin PR 802 (FR-044-AC-14/15 test backing)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@589a073; spec/functional/FR-044, spec/functional/FR-108, spec/matrix.md, rust/crates/quoin-measurement, rust/crates/quoin-measurement-graph"
review_set: subset
---

# Gap analysis of quoin PR 802

## Summary

Ticket: PLAT-164. This review is planless. It is scoped to the criteria the PR adds or edits:
FR-044-AC-14, FR-044-AC-15 and FR-108-AC-22. Evidence comes from `quoin matrix --repo . --json`
(quoin 0.30.0) at 589a073.

- FR-044-AC-14 is tagged by `tc_479_003` and `tc_164_001`.
- FR-044-AC-15 is tagged by `tc_164_002` and `tc_164_003`.
- All four use the `#[trace("FR-044-AC-1x", "TC-214x")]` attribute form.
- FR-108-AC-22 is untagged. Its harness is not built yet (🚧), so the matrix status is
  correctly not ✅.

Plan completion: not assessed

## Verdict

**CONDITIONAL.** The main paths of both new criteria have tests. Three stated clauses have no
test: the campaign runner stating `undeclared`, a pair where one side states no profile, and
the portfolio applying the rule. The matrix row for TC-2140 also cites two tests whose tags
belong to another row. Semantic review was skipped because it was not opted in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-044-AC-14 says the campaign runner states `undeclared`, but no test asserts the profile a campaign publishes. The line it changed is also uncompiled under default features (code-review FND-001). Add a `campaign`-feature test that reads the published collection's `verificationStack.buildProfile` | rust/crates/quoin-measurement/src/campaign/run/collection.rs:145 |
| FND-002 | medium | FR-044-AC-15 says "an unstated profile differs from any stated one", but `tc_164_002` compares only release against debug. No test covers a historical collection with no profile paired with a stated one, the case every pre-profile history hits first | rust/crates/quoin-measurement/tests/tc_479_fr044_fr045_criteria.rs:1256 |
| FND-003 | medium | FR-044-AC-15 says "The portfolio comparison applies the same rule", but no test drives the portfolio. A regression back to the plan-less call in portfolio/build.rs would pass every test | rust/crates/quoin-measurement/src/portfolio/build.rs:167 |
| FND-004 | low | Row TC-2140 cites `tc_479_020` and `tc_479_004` as backing FR-044-AC-14. Both are tagged `FR-044-AC-1`/`TC-1003`, so the hand-written cell claims backing that the computed matrix does not record. Either tag them `TC-2140` or drop them from the cell | spec/matrix.md:546 |
| FND-005 | low | In `tc_479_003`, the refusal of an unknown (`instrumented-ish`) or absent profile asserts only that some error occurred, not that the refusal names `buildProfile`. Any unrelated refusal would pass | rust/crates/quoin-measurement/tests/tc_479_fr044_fr045_criteria.rs:481 |

## Coverage

- Criteria in scope: FR-044-AC-14 and FR-044-AC-15 are tagged (`static_status` tagged).
  FR-108-AC-22 is untagged and pre-existing 🚧.
- Untagged criteria introduced by the PR: none.
- Reverse gap: the `profile_reason` code in compare.rs, the plan member parser, and the
  intake change each have an owning AC.
- Stubs: none.
- Semantic review: skipped.
- Plan completion: not assessed

## Examined scope

```yaml
clean: false
scope:
- {id: FR-044-AC-14, path: spec/functional/FR-044-plan-governed-measurements.md, role: examined, excerpt: "the campaign runner launches arbitrary processes and states it"}
- {id: FR-044-AC-15, path: spec/functional/FR-044-plan-governed-measurements.md, role: examined, excerpt: "an unstated profile differs from any stated one ... The portfolio comparison applies the same rule."}
- {id: FR-108-AC-22, path: spec/functional/FR-108-measurement-verdict-checker.md, role: examined, excerpt: "a harness that skips the live write for a debug build says so on stderr with the reason"}
```
