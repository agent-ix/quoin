---
id: SR-200
title: "Gap analysis — PLAT-1162 evidence gates without apparatus inventories (quoin PR 764, re-review)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-110-protected-apparatus.md (Plan intake, FR-110-AC-1), spec/matrix.md (FR-108, FR-110, TC-1810), rust/crates/quoin-measurement/src/plans.rs, rust/crates/quoin-measurement/src/plans/apparatus.rs, rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-110", type: references}
---

## Summary

Ticket: PLAT-1162. PR head: `be4803e6`. A planless gap analysis scoped to the PR diff. Plan completion was not assessed. It does not replace SR-197. SR-197 FND-004 is still open at this head: the FR-108 matrix row credits PLAT-1162 to `tc_975_010` and `tc_975_011`, which are TC-1819's and TC-1820's tests (spec/matrix.md:135). It is not filed again here.

Each FR-110-AC-1 clause added by this PR has a test:

- A non-retired gate missing `ground_truth_kind`, `statistical_design` or any of its seven members is refused, naming the member: tc_975_024.
- A gate missing `negative_controls` is refused: tc_975_001.
- A retired gate with no controls loads: tc_975_026.
- An `apparatus-edit` control with no apparatus is refused: tc_975_001.
- A complete gate with no apparatus list loads: tc_975_001 and tc_975_022.
- A gate with an `execution_procedure` and no apparatus is refused: tc_975_025.
- The Analyze-shaped MP-001 passes the locked EA schema and Quoin, and its own `count`/`eq 0` rule decides retained evidence: tc_975_022 and tc_975_027.

All production code in the diff traces to FR-110's Plan intake.

## Verdict

**PASS with one low finding.** The ACs are covered by tests that would fail if the behaviour broke. SR-197 FND-004 is still open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `tc_975_022` is tagged `FR-108-AC-4`, and the TC-1810 row now lists `FR-108-AC-4` too. But tc_975_022 asserts only `NoCollections` with no observation, so it recomputes nothing and judges no population, which is what FR-108-AC-4 is about. FR-108-AC-4's own Verification column does not list TC-1810. tc_975_023 and tc_975_027 do exercise AC-4 (incomplete is inconclusive, `count` recomputed from `matched`). | rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:211-212; spec/matrix.md:630 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-110-AC-1 | examined | Every clause is backed by a test (list above) |
| FR-110 Plan intake bullets 1-2 | examined | Agree with the code and with EA FR-024-AC-8 |
| FR-108-AC-4 | examined | Bound correctly by tc_975_023 and tc_975_027; tc_975_022's binding is wrong (FND-001) |
| TC-1810 matrix row | examined | Names tc_975_022..027 without ambiguity; extra FR-108-AC-4 link (FND-001) |
| FR-108 matrix row | examined | Still cites tc_975_010/011 (SR-197 FND-004, open) |
| EA FR-024-AC-8 | context_only | Quoin's trigger (procedure or apparatus-edit) matches |
| Production code with no owning requirement | examined | None |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | On main, after #785 (PLAT-847), the repository's rust-style skill says a `/// Trace:` doc line binds nothing. A test backs a matrix row only through `#[trace("<AC>", "<TC>")]`, and main's tc_975_001..021 were retagged that way. The PR's six new tests tc_975_023..028 still carry only `/// Trace: FR-110-AC-1[, FR-108-AC-4]`. Once the PR is merged with main (which it must be, since it conflicts), none of them backs FR-110-AC-1, FR-108-AC-4 or TC-1810. TC-1810's row then rests on tc_975_001 and tc_975_022 alone, and the new gate-definition, procedure, retired-gate and verdict clauses have no bound test. Each needs `#[trace("FR-110-AC-1", "TC-1810")]` (plus `FR-108-AC-4` where it applies). | rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:281,412,479,499,515,564 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | At 0d07018c tc_975_022 still carries `#[trace("TC-1810", "FR-110-AC-1", "FR-108-AC-4")]` and still asserts only `Reason::NoCollections`. It recomputes no estimate and judges no population, so the FR-108-AC-4 tag is still wrong. Low; does not block merge on its own |
