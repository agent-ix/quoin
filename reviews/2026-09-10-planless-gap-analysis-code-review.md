---
id: SR-149
title: "Code review — gap-analysis planless by default (#365, PR #366)"
type: SpecReview
analysis: code-review
scope: "skills/gap-analysis/, tests/gap-analysis-skill.test.ts, README.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/TM-001", type: references }
---

## Summary

Reviewed the nine files on `365-planless-gap-analysis` (PR #366): the `gap-analysis` skill
and its six references, the new `tests/gap-analysis-skill.test.ts`, and the README section.
The change is prose plus one test file — no source modules, no mocks, no database access —
so the Python, Rust and React lanes do not apply and the applicable checks are Integrity,
Completeness (test code), Code-Test Alignment and the reverse gap. No `high` finding. Four
of the five findings were fixed on the branch during the review; FND-002 stands and is
recorded rather than silently accepted.

## Verdict

**CONDITIONAL** — two `medium` findings, one of which (FND-002, an untracked test file) is
left open with a stated reason; no `high` finding.

## Findings

| ID      | Severity | Summary                                                                             | Refs                                                            |
| ------- | -------- | ----------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| FND-001 | medium   | Test assertions were coupled to Prettier's line wrapping, not to the claims — fixed | tests/gap-analysis-skill.test.ts:31                             |
| FND-002 | medium   | New test file carries no `TC-` tracking tag and mints no Test Matrix row — open     | tests/gap-analysis-skill.test.ts:1                              |
| FND-003 | low      | Stale numbered cross-references survived the step reordering — fixed                | skills/gap-analysis/references/step-3-matrix-verification.md:54 |
| FND-004 | low      | `## Modes` said the opt-in semantic review "runs in full" in planless mode — fixed  | skills/gap-analysis/SKILL.md:42                                 |
| FND-005 | low      | Two prose lines exceeded the files' wrap width after editing — fixed                | skills/gap-analysis/SKILL.md:137                                |

### FND-001 — assertions coupled to the wrap, not the claim

Fourteen assertions matched across newlines (`"The\npresence of a `plan/` directory…"`) or
used `\s+` to bridge one. Prettier owns the wrapping in these files, so re-flowing a
paragraph — or changing `proseWrap` — would have failed tests whose subject had not changed,
and the usual repair for that is to weaken the assertion. **Fixed**: every file is read
through a whitespace-flattening reader, so the coupling sits on the sentence. Re-verified
red against `main`'s skill (12/12 fail) and green on the branch (12/12 pass).

### FND-002 — the test file is untracked (open)

`quire coverage` classes a test carrying no declared trace id as an untracked symbol, and
the repo's convention is one `TC-NNN` row in `spec/matrix.md` per unit test (TC-271 for
`tests/skills-vocab-drift.test.ts`, TC-272 for `tests/quire-types-conformance.test.ts`). The
new file has neither.

It is left open rather than fixed because **no requirement owns it**. quoin's spec states no
behavioural requirement for any skill's workflow; NFR-005 covers vocabulary coupling only.
Minting a `TC-` row means first authoring the requirement that a skill's audit is
repository-driven, which is spec work, not a tag. The precedent is the same: `quoin#202`'s
`tests/skill-contracts.test.ts` — the contract gate for all 18 skills — is equally untracked.
Recorded here so the absence is a decision on the record rather than an oversight.

### FND-003 — stale step numbers

Reordering the audit left two numbered references pointing at positions that no longer exist:
`step-3-matrix-verification.md:54` cited "Step 6" after the reference headings dropped their
numbers, and `SKILL.md`'s list stayed 0-indexed with the plan overlay labelled "Step 4" while
living in `step-2-plan-completion.md`. **Fixed**: the steps are no longer numbered at all;
each is labelled by when it runs (`Always`, `Only when the user opts in`, `Only with an
explicit plan`), which also removes the numbering coupling from the test.

## Coverage

- Lanes run: Integrity, Completeness (test code), Code-Test Alignment, reverse gap. Python /
  Rust / React lanes not applicable — the change adds no source module.
- Files reviewed: 9 (8 markdown, 1 TypeScript test).
- Test stubs, weak-only assertions, skip markers, mock-only tests: 0. No mock is used; the
  test reads real files and asserts on their content.
- Suppressed warnings, lowered thresholds, coverage pragmas: 0.
- Gates run: `make lint` clean; full suite 1181/1181 (105 files) via the `test-with-quire`
  inner target with quire 0.31.0.
- Gates not run, with reasons: `make test` (quire evidence-overlay relock) and `make validate`
  (duplicate `## Impact Scenarios` heading in `spec/assurance/AP-201-finding-quality.md`)
  fail on pre-existing repository state; both reproduce on a clean tree with this branch's
  changes stashed.
- Near-miss caught by an existing gate, before the first commit: the rewritten SKILL.md
  frontmatter used a plain scalar containing `: `, which is invalid YAML and would have made
  the whole skill unloadable. `tests/skill-contracts.test.ts` failed on it, which is exactly
  the drift that test was written for.
