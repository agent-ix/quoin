---
id: SR-20261111195807
title: "Gap analysis of quoin PR 813 (FR-039 AC-13..17)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@25d66ce; spec/functional/FR-039-mutation-score-threshold.md, spec/matrix.md, spec/functional/FR-115-evidence-backed-test-matrix.md, tests tagged FR-039-AC-13..17"
review_set: subset
---

# Gap analysis of quoin PR 813

## Summary

Ticket: PLAT-394. Planless. Plan completion: not assessed. The scope is the PR diff only.
`quire matrix --scope .` shows FR-039-AC-13..17 each `tagged`:

| Criterion | Test |
| --- | --- |
| AC-13 | `tc_adapters_behaviour.rs:225` |
| AC-14 | `tc_audit_ladder.rs:800` |
| AC-15 | `tc_audit_ladder.rs:822` |
| AC-16 | `tc_audit_ladder.rs:848` |
| AC-17 | `tc_3102_surviving_mutants_skill.rs:27` |

None of them reports `criterion-tagged-outside-row`. The ids are free:

- TC-3100..3102 and FR-039-AC-13..17 appear nowhere on origin/main (5f056b84).
- They appear in neither open PR #811 nor #812.
- #810 (merged) does not use them.
- The PLAT-1401 worktree uses TC-3000..3005 only.

The matrix rows for TC-3100..3102 are present. The PR adds no code without an owning requirement.

## Verdict

**CHANGES REQUESTED (three medium, two low).** Every new criterion has a tagged test. The gaps
are:

- FR-039 overstates what a suite-granular survivor proves (FND-001)
- AC-17's only test is a text pin (FND-002)
- FR-115's enumeration of finding kinds is now false (FND-003)
- two AC-15 clauses are untested (FND-004)
- one matrix row cites a test that does not exist (FND-005)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Binding granularity is the suite, but FR-039 and the finding text claim more. Every survivor in any mutation-scored entry of a bound suite is reported against **every** obligation bound to that suite, up to 25 each. So one survivor in an unrelated function repeats under N criteria. FR-039 says "per kept survivor of the bound suites", which is literally true. It then says "the finding says that the criterion's tests, or the criterion itself, do not demand that behaviour", and the summary says "the tests bound to it do not detect". Neither holds when the mutated function is not one the criterion's tests exercise. The skill then tells an agent to write a property for each survivor under that criterion's row_id, which mis-traces generated tests. Either state the suite granularity and the duplication plainly in FR-039 and in the finding/skill wording ("a survivor in a suite bound to X"), or bind per symbol before shipping the skill routing | spec/functional/FR-039-mutation-score-threshold.md:91-101, rust/crates/quoin-auditor/src/audit/scores.rs:70-104, skills/spec-correctness/references/surviving-mutants.md:45-47 |
| FND-002 | medium | TC-3102 is a static pin on skill prose. It passes if `SKILL.md` names the reference file and the reference contains five substrings ("emit one property", "`row_id`", and so on). It would still pass if the reference said the opposite of AC-17, and it says nothing about whether the skill "directs a test at each" survivor. That fails the value test: it pins text, not behaviour. FR-038 precedent verifies agent-facing skills at the eval layer. Delete the test and verify AC-17 by Inspection, or by an eval scenario that feeds survivor findings and checks a property is emitted per row_id | rust/crates/quoin-core/tests/tc_3102_surviving_mutants_skill.rs:24-40, spec/functional/FR-039-mutation-score-threshold.md:122 |
| FND-003 | medium | FR-115 says "That is all 11 FindingKind values this FR reads (`quoin-finding-types::FindingKind::ALL`)" and maps each one to a status. `ALL` is now 12, and `surviving-mutant` is in neither the list nor the status table. The code falls through to `undischarged` (`matrix::bucket`), which is right, but the spec claim is now false. Add `surviving-mutant` to FR-115's undischarged list and update the count | spec/functional/FR-115-evidence-backed-test-matrix.md:59-70, rust/crates/quoin-finding-types/src/finding.rs:173-187 |
| FND-004 | low | FR-039-AC-15 has five clauses: floor met, undeclared, unmeasured, skipped symbol, unbound suite. `no_survivor_findings_unless_the_floor_is_missed` asserts three of them. The unmeasured case and the not-bound-suite case have no assertion, so the `bound.contains` filter in `surviving_mutant_findings` could be deleted and every test would still pass. Add both cases | rust/crates/quoin-auditor/tests/tc_audit_ladder.rs:822-845, spec/functional/FR-039-mutation-score-threshold.md:120 |
| FND-005 | low | The TC-3100 row in `spec/matrix.md` names `quoin-evidence` `src/adapters/cargo_mutants.rs :: a_missed_mutant_is_kept_with_its_line_and_description_and_a_kill_is_not`. No such test exists. The tagged test is `tests/tc_adapters_behaviour.rs :: tc_3100_a_missed_mutant_is_kept_with_line_and_description_bounded`. Fix the row text | spec/matrix.md:476 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-039-AC-13, path: spec/functional/FR-039-mutation-score-threshold.md, role: examined, excerpt: "The cargo-mutants adapter records, per function, each missed or timed-out mutant with its line and description, at most 25 per function and 200 characters each, and a killed mutant is not recorded."}
- {id: FR-039-AC-14, path: spec/functional/FR-039-mutation-score-threshold.md, role: examined, excerpt: "When the floor is missed, the audit reports one surviving-mutant finding per recorded survivor of a mutation-scored symbol bound to the obligation, at medium, carrying path, line and symbol."}
- {id: FR-039-AC-15, path: spec/functional/FR-039-mutation-score-threshold.md, role: examined, excerpt: "No surviving-mutant finding is produced when the floor is met, undeclared or unmeasured, for a skipped symbol, or for a suite not bound to the obligation."}
- {id: FR-039-AC-16, path: spec/functional/FR-039-mutation-score-threshold.md, role: examined, excerpt: "An obligation reports at most 25 surviving-mutant findings."}
- {id: FR-039-AC-17, path: spec/functional/FR-039-mutation-score-threshold.md, role: examined, excerpt: "The spec-correctness skill names surviving-mutant findings, grouped by row_id, as an optional input and directs a test at each."}
- {id: FR-039-AC-1, path: spec/functional/FR-039-mutation-score-threshold.md, role: examined, excerpt: "no floor, no finding (regression check)"}
- {id: FR-115, path: spec/functional/FR-115-evidence-backed-test-matrix.md, role: examined, excerpt: "That is all 11 FindingKind values this FR reads"}
```
