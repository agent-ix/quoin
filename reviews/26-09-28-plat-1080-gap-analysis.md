---
id: SR-175
title: "Gap analysis — PLAT-1080 evidence-backed test matrix (FR-115)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@1cb9159127bb7588f7960794905ff208cbc9389a; PR 650 diff vs merge base e8679e9: FR-115 (AC-1..15, CON-1..4, CR-001..003), spec/matrix.md (FR-115, FR-021, US-005 rows, TC-1964..TC-1978), spec/evals.md (TC-EV-013), spec/log.md, FR-020, FR-021, US-005, spec/spec.md, and the Rust sources and tests listed in SR-174"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/TM-001"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "reviews"
---

# SR-175: Gap analysis — PLAT-1080 evidence-backed test matrix (FR-115)

## Summary

Ticket: PLAT-1080. This was a repository-driven audit of FR-115 against its tagged tests and
code at `1cb9159`. Plan completion was not assessed.

All 15 acceptance criteria have backing `tc_1080_*` tests. Each test carries `/// Trace:` lines
for its criteria, and TC-1964..TC-1978 each have exactly one matrix row. CON-3 and CON-4 are traced.
CR-003's corrected `coverage` wire shape matches the code: a bare array, omitted when empty. The
TC-EV-013 and US-005 matrix edits are accurate. The `quire validate` error set on the changed spec
files is identical to main's, and FR-115 itself validates.

One spec contradiction remains. FR-020 still requires `matrix` to be a workflow launcher.

## Verdict

**CONDITIONAL**. There is one medium finding: a spec contradiction the behaviour change introduced.
It is a small text fix and should land in this PR. The two low findings are traceability polish.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-020 still states that "The CLI SHALL expose the `review`, `matrix`, and `to-plan` workflow launchers". Its AC-1 ("`review`, `matrix`, and `to-plan` are recognized as the bundled workflow launchers") stays ✅ in the matrix (TC-065). This PR makes it false, and it contradicts FR-115-AC-13. FR-021 received a CR note and FR-020 did not. US-005 Context and spec.md (lines 28, 46, 95, 117) still describe matrix as a governed workflow. | spec/functional/FR-020-resolve-workflow-skills.md:16,24,32,43; spec/matrix.md:340; spec/usecase/US-005-start-gated-spec-workflow.md:24; spec/spec.md:28 |
| FND-002 | low | `quire coverage` lists FR-115's functional-coverage row as unbacked. The tests trace `FR-115-AC-n` only, and no symbol carries `FR-115` or any of TC-1964..TC-1978, which are the row's own target ids. There is also an unmatched bare `FR-115` tag at tc_1080_matrix.rs:122, from a doc sentence. | spec/matrix.md:266; rust/crates/quoin-assurance/tests/tc_1080_matrix.rs:122 |
| FND-003 | low | FR-115-CON-1 declares `Validation: Test`, but no test traces it and the FR-115 functional-coverage row omits it. That row's own rule ("criteria absent here are verified by a method that produces no test") therefore misstates how CON-1 is verified. | spec/functional/FR-115-evidence-backed-test-matrix.md (CON-1); spec/matrix.md:207,266 |

## Coverage

- Matrix verification: `quire coverage --scope . --json` (quire 0.33.0, engine 0.47.1) found no
  status lies, unbacked TC rows, no-symbol rows or shared trace ids for FR-115 or TC-1964..TC-1978.
  Repo-wide totals are backed 534 of 2212, and that repo-wide state is pre-existing.
- Tagged tests: `tc_1080_001..009` (quoin-assurance), `tc_1080_100..102` (quoin-core boundary),
  `tc_1080_200..206` (quoin-cli, real subprocess and real store), `tc_1080_300` (flow.rs), and
  `coverage_projects_the_engine_matrix_in_its_own_spelling` (quire.coverage projection).
- Reverse gap: every new symbol has an owner (FR-115). The `evidence::audit::assemble` extraction
  is owned by FR-115-AC-14 and FR-032.
- Stubs and inflation: none. A 14-mutant run killed every mutant (SR-174).
- Semantic review: the mutation run and the main-vs-branch byte comparison stood in for it. No
  subagent fan-out.
- Plan completion: not assessed
