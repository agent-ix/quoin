---
id: SR-177
title: "Gap analysis — PLAT-1083 stop authoring the Test Matrix"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@1aceb1552747024441f1ff7ddc1cd024d5a2a21f; PR 653 diff vs origin/main d79c3b9 against PLAT-1083 intent: skills/, .claude/skills/, templates/semantic-module/, spec/ (FR-028, FR-076, FR-082, FR-083, NFR-005, matrix.md, log.md)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-082"
    type: "reviews"
---

# SR-177: Gap analysis — PLAT-1083 stop authoring the Test Matrix

## Summary

Ticket: PLAT-1083. PR agent-ix/quoin#653 at `1aceb15`. A planless audit of the PR against the ticket intent.
The intent: skills and templates stop telling agents to author or maintain a Test Matrix, they teach binding
tests by criterion id, there is no sweep, existing matrices stay, and the `TestMatrix` archetype stays valid.
Each intent item is met. spec-matrix is reduced to "read the computed matrix, tag the test" and its asset
templates and TC examples are deleted. gap-analysis reads `quoin matrix` / `quire matrix` and FAILs on an
untagged criterion. specify, spec-review, spec-app-review, spec-correctness, spec-object-review and rust-style
all teach criterion-id trace tags and method-only `Verification` cells. The template renders no
`spec/matrix.md`, and nothing rendered links one. quoin's own `spec/matrix.md` is kept and only amended for the
restated and withdrawn rows.

## Verdict

**CONDITIONAL**. One medium finding: the rendered template loses its only record of two known coverage gaps.
There are no high findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The deleted rendered matrix was the template's only record of why NFR-001-AC-1 and NFR-001-AC-2 are not covered: "skip count read by eye" and "validator-absent path not driven" (TC-029/TC-030 `🚧`). The rendered tests carry no `pytest.mark.trace` for either criterion, measured by comparing the 32 marker ids with the criterion ids in the rendered spec. So a fresh render now shows both as `untagged`, with no reason anywhere in the rendered tree. Under the new gap-analysis verdict rule, "any untagged criterion → FAIL", a freshly rendered repository fails its first gap-analysis. FR-082's own rationale says a template should not make "the first act in a new repository a repair". One fix is to state the two gaps in NFR-001's text. Another is to tag the conftest-driven checks, or to make the criteria honest. | templates/semantic-module/{{cookiecutter.repo_name}}/spec/non-functional/NFR-001-verification-without-skips.md:51-54, templates/semantic-module/{{cookiecutter.repo_name}}/tests/ |
| FND-002 | low | Out-of-repo residue, not blocking: `@agent-ix/ix-spec-workflows` 0.1.3 is a quoin dependency (`package.json`). It still ships `spec/matrix/SKILL.md` ("Build and update requirements test matrices") with a matrix workflow, and its to-plan template's T-3 reads "Requirements matrix or review report updated". quoin's `package.json` description still says quoin-cli launches a `quoin matrix` workflow, which has been stale since FR-115. These tell agents to maintain a matrix outside this PR's scope and need a follow-up in that repository. | package.json:5, node_modules/@agent-ix/ix-spec-workflows/spec/matrix/SKILL.md |

## Coverage

- Reconciliation: quire matrix (quire 0.34.0 release build, cli c739677) — no run evidence read. `quoin matrix` in this worktree is unreleased, and FR-115 is in no tag yet.
- Criteria in the PR's spec scope: FR-082-AC-1..6 `untagged`, FR-076-AC-10 `untagged`, NFR-005-AC-1 `method-without-symbol`, FR-083-AC-7 absent (withdrawn). The untagged state is honest, because the Node render harness retired. The quoin matrix rows record it as `🚧`.
- Rendered template: 32 test functions carry 32 `pytest.mark.trace` markers. No marker names an id outside the rendered spec. Untagged: NFR-001-AC-1, NFR-001-AC-2, StR-001-VC-1, StR-001-VC-2 (whether VC ids mint depends on the module).
- Residual grep over skills/, .claude/skills/ and templates/: no instruction to write matrix rows, TC ids or Status markers remains, apart from the leader-accepted spec-to-plan plan-local TC ids.
- Gate: coder log `plat-1083-quoin-ci.log` ends `head=ebca3a1e1d3902ee1e1f23130cea3353d08ca898 exit=0`. `git diff ebca3a1 1aceb15` touches only FR-028. CI checks (cla gate) are SUCCESS, and the PR is MERGEABLE.
- Untraced behaviors / stubs: 0 (docs-only PR)
- Semantic review: skipped (docs-only)
- Plan completion: not assessed

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The fix's new NFR-001 Verification paragraph says `tests/conftest.py` "is inspected to confirm every absent-tool path fails naming its install command". conftest holds only the engine and grammar-package paths (`pytest.fail(QUIRE_MISSING)`, `SEMANTIC_CORE_MISSING`). The schema-toolchain path is in `tests/test_schema_emission.py:28-31` (and `scripts/generate-schemas.mjs:75-78`), so an inspector following the text misses one of AC-2's three tools. Separately, AC-1 is verified by reading a run's skip summary, which is closer to `Manual` than `Inspection`. Both methods compute `method-without-symbol`, so this is precision only. | templates/semantic-module/{{cookiecutter.repo_name}}/spec/non-functional/NFR-001-verification-without-skips.md:49-52 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The round-2 fix sets rendered NFR-001-AC-1's Verification cell to `Manual`. Under the pinned spec-artifacts-process v0.27.0, lint rule `acceptance-criterion-verification-method` (archetypes FR/NFR/interface) does not admit `Manual`. The manifest says "`Eval` and `Manual` are deliberately absent". `quire lint --module <v0.27.0>` warns on row 1. The matrix is unaffected, since `Manual` is still `no_source_symbol` and computes as `method-without-symbol`. My own FND-003 text suggested `Manual`, and that suggestion was wrong on this point. `Inspection` is both lint-allowed and `no_source_symbol`. Revert AC-1 (and the M-1 metric and the FR-082 CR-001 sentence) to `Inspection`. | templates/semantic-module/{{cookiecutter.repo_name}}/spec/non-functional/NFR-001-verification-without-skips.md:60, templates/semantic-module/{{cookiecutter.repo_name}}/spec/non-functional/NFR-001-verification-without-skips.md:39, spec/functional/FR-082-generated-governance-tree.md:84 |

## Dispositions

Round 1, reviewed at `86f7c36cf8b592c59c106074f42f82b283f1eaf5`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 86f7c36: rendered NFR-001-AC-1 and AC-2 (and the M-1/M-2 metrics) are now `Inspection`, with the reason stated in NFR-001 and in FR-082 CR-001. Measured: `quire matrix` (0.34.0) over the template tree reports both as `method-without-symbol`, and no criterion reads `untagged` (StR VC ids do not mint). This is honest. No test asserts either criterion. AC-2's absent-tool paths are real `pytest.fail` calls you can read in the source, and AC-1 is a read of the run's summary. See FND-003 for the precision of the new text. |
| FND-002 | deferred | ix-spec-workflows PR (tracked under PLAT-1083). The in-repo part is fixed in 86f7c36: the `package.json` description no longer says quoin-cli launches a `quoin matrix` workflow. |

Round 2, reviewed at `a50fd540ad9285ef00bff1513076f98827ab8c8e`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-002 | accepted-no-change | agent-ix/ix-spec-workflows is archived and read-only by the owner's choice (`gh repo view` reports `isArchived: true`). Its matrix workflow has been unreachable since quoin#650 removed the only launcher: `quoin-cli/src/flow.rs` `tc_1080_300_matrix_is_not_a_flow` asserts the flows are exactly `["review", "to-plan"]` and that `matrix` is refused. No change is possible or needed. |
| FND-003 | fixed | a50fd54: NFR-001's Verification text and FR-082 CR-001 now name all three absent-tool loci: `tests/conftest.py` (engine, grammar package), `tests/test_schema_emission.py` (schema toolchain) and `scripts/generate-schemas.mjs`. AC-1's method changed to `Manual`, and that change introduces FND-004. |

Round 3, reviewed at `4f3bf41daf14a41c0f75fcbf000d32d13dba2d2f`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | 4f3bf41: rendered NFR-001-AC-1, the M-1 metric and the FR-082 CR-001 sentence now read `Inspection`, and no `Manual` remains. `quire lint` against the exported spec-artifacts-process v0.27.0 module emits no warning on NFR-001. It warned on `Manual` at a50fd54, which serves as the positive control. `quire matrix` 0.34.0 over the template reports NFR-001-AC-1 and AC-2 as `method-without-symbol`, and no criterion as `untagged`. |
