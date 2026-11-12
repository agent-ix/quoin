---
id: SR-20261112201003
title: "Code review of quoin PR 835 (delete hand-kept spec/matrix.md and evals tables)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@843068e4; spec/matrix.md, spec/evals.md, spec/index.md, spec/review.md, spec/spec.md, rust/crates/quoin-core/tests/tc_1045_spec_integrity.rs, .claude/skills/rust-style/SKILL.md, skills/spec-matrix/SKILL.md, skills/spec-correctness/**, skills/spec-fuzz/**"
review_set: subset
---

# Code review of quoin PR 835

## Summary

Ticket: PLAT-1496. Scoped to `git diff origin/main...HEAD` at 843068e4, with the rust-review
lane folded in. Tools: quire 0.43.0 (engine 0.58.0), quoin built from the branch (0.31.0-2-g843068e4).

Checks run: `quire validate` over the changed spec files exits 0. Over all of `spec/`, the only
failure left is the FR-113-AC-6 row-width error, which is already on main. Main also fails on
spec/matrix.md, so this PR removes one failing document. `cargo test -p quoin-core --test
tc_1045_spec_integrity` passes all 12 tests. Each of the six deleted tests (tc_1045_001/002,
tc_119_001/002, tc_1045_009/010) read only `spec/matrix.md` or a planted table string. None
tested a product behavior, so no behavior lost its test. No compatibility layer or replacement
table was added.

## Verdict

**PASS WITH FINDINGS (two medium, two low).** The deletion breaks no tool, test or validation.
However, the `TC-` ids that matrix.md minted are still cited in about 1,750 places across spec,
plan and test markers, and they now resolve to nothing. The rust-style text this PR adds says
TC ids do not belong on the marker.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Deleting matrix.md left 236 `relationships` targets in 42 plan task files pointing at `ix://agent-ix/quoin/TC-NNNN` ids. The only document that minted those ids was matrix.md. `quire validate` does not resolve the targets, so nothing reports them. Delete the TC `verifies` edges, or retarget each one to the criterion id the matrix row cited | plan/PLAN-002-intervention-experiment-evidence/tasks/TASK-006-record-contract.md:13 |
| FND-002 | medium | The PR's new rust-style text says only requirement criteria belong on `#[trace]`. Yet 723 existing markers in 135 rust files still carry a dead `"TC-NNNN"` argument. The FR/NFR/StR verification cells also hold 789 `TC-` citations, such as `Test (TC-111)`, that now name nothing. Strip the TC arguments and the citations mechanically, in this PR or in one follow-up sweep | rust/crates/quoin-assurance/src/authored/build/tests.rs:270, spec/functional/FR-029-consume-the-quire-json-contract.md:59 |
| FND-003 | low | spec/review.md now says "every FR/NFR maps to a named tests/ case, found by the computed matrix" and "the computed matrix names the case". Both are false: `quoin matrix` reports 552 criteria as untagged. Drop both claims, or state that the computed matrix reports the untagged criteria | spec/review.md:36, spec/review.md:147 |
| FND-004 | low | The TC-EV-013 cell says "covered by TC-1964..TC-1978". Those ids are gone. Cite FR-115 criteria instead. The evals.md body also still calls itself "this matrix" and points at an `evals/` harness and `evals/scenarios/index.mjs`, neither of which exists in the repo. The harness reference was already broken on main | spec/evals.md:99, spec/evals.md:32 |

## Examined scope

```yaml
clean: false
scope:
- {id: tc_1045_spec_integrity.rs, path: rust/crates/quoin-core/tests/tc_1045_spec_integrity.rs, role: examined, excerpt: "six deleted tests read only spec/matrix.md or planted table strings; 12 remaining tests pass"}
- {id: TM-002, path: spec/evals.md, role: examined, excerpt: "type: index; Contents added; Functional Requirement Coverage and Test Case Summary tables deleted; 46 TC-EV scenario rows intact"}
- {id: rust-style, path: .claude/skills/rust-style/SKILL.md, role: examined, excerpt: "Tag by criterion id alone. There is no hand-kept matrix and no TC- id to mint"}
- {id: spec-matrix, path: skills/spec-matrix/SKILL.md, role: examined, excerpt: "Do not add such a document; delete it and tag by criterion id."}
```
