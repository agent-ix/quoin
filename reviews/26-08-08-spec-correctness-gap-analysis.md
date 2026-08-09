---
id: SR-003
title: "Gap-analysis review of the spec-correctness skill (US-011, FR-028)"
type: SpecReview
analysis: gap-analysis
scope: "FR-028; US-011; EV-050..EV-053; skills/spec-correctness/; tests/props/; the 115 tracking tags across tests/*.test.ts; spec/matrix.md TM-001; spec/evals.md TM-002"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-028"
    type: "reviews"
  - target: "ix://agent-ix/quoin/TM-001"
    type: "references"
---

## Summary

Post-implementation gate over the Phase C work released as `@agent-ix/quoin@0.12.0`:
the `spec-correctness` skill, FR-028 + US-011, the four eval scenarios, the 28
generated property tests, and the 115 tracking tags added across the suite.

**Step 1 (plan completion) is not applicable** — this work has no `plan/` bundle.
It was specified and reviewed FR-first rather than planned into tasks, so there is
no task status to assert. That is a deviation from the usual gate, and it is
recorded rather than waved through.

**Step 2 (matrix verification) passes on its own terms and exposes one structural
gap.** No FR or NFR row in `spec/matrix.md` claims ✅ coverage it cannot back with
a tracking tag: 35 requirements were checked mechanically, 109 of 130 criteria
resolve to a tagged test, and the 21 that do not are each verified by a method
that produces no test (help rendering delegated to `@oclif/core`, FR-028's own
agent behavior via EV-050…EV-053, one accepted limitation, six StR demonstration
criteria). Before this work that number was **0** — the matrix traced in prose
only, so nothing was reconcilable by grep at all.

The structural gap is that **StR-001…StR-006 have no rows in the matrix**. Six
stakeholder requirements carrying validation criteria sit entirely outside it.
That predates this work and is not caused by it.

**Step 3 (underspecified code) is clean.** The change touched no `src/` file, so
there is no new behavior to own. The one new runtime artifact is the `fast-check`
devDependency, and it is worth being explicit that a human added it: FR-028-AC-11
requires the _skill_ to install nothing and report a remedy instead, which is
exactly what it did — the queue recorded the remedy and the dependency was added
by decision, not by the tool.

**Step 4 (semantic review) found one real divergence**, and it is the reason this
gate returns FAIL. FR-005's Behavior says the CLI "SHALL raise an error naming the
unknown command **and including the root usage**". It does not. `quoin
bogusgarbage` produces `Error: command bogusgarbage not found` with no usage text,
and no test in the suite asserts usage at all. The generated property test for
FR-005-AC-1 asserts that any unknown bareword is rejected and that the message
names the command — both true — but **not** the clause the criterion actually
turns on, so it passes while the criterion is unmet.

This is drift from the FR-026 oclif migration, which removed the hand-rolled
dispatcher that used to print usage; FR-005 kept an acceptance criterion the
migration invalidated. It is a spec-versus-code question for the author to settle:
either the runner should surface usage, or FR-005-AC-1 should describe what the
runner does. **This review does not propose the wording.**

Worth recording as a positive: FR-028-AC-7 ("accepting a queued test leaves its
tracking tag byte-identical") was verified mechanically rather than asserted. The
`Trace:` and `row=` tags were extracted before and after acceptance and diffed.

## Verdict

**FAIL at time of review** — one `high` finding: an acceptance criterion whose
stated oracle was neither met by the implementation nor asserted by the test that
claimed it.

**All four findings have since been resolved** (see Resolution below). The gate is
retained at FAIL as the record of what the review found; re-running it against the
current tree returns PASS.

## Resolution

| ID      | Resolution                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| FND-001 | **Fixed in the implementation, not the spec.** `src/cli.ts` gained `rootUsage`, `isUnknownCommand` and `withRootUsage`; `main()` appends the root usage to an unknown-command error, and `bin/quoin.js` now routes through `main()` so the shipped CLI behaves the same as the tested path. `quoin bogusgarbage` now lists the commands and exits 2. The property test asserts the usage clause it previously skipped. |
| FND-002 | Fixed. `spec/matrix.md` gained a **Stakeholder Requirement Coverage** section for StR-001…StR-006. Recording them surfaced a second, smaller gap, now visible rather than hidden: **StR-004-VC-1 is ⚠️ Partial** — no eval exercises workflow resume/advance/gate-acknowledge.                                                                                                                                         |
| FND-003 | Fixed. FR-027-AC-6 is now tagged on `cli.test.ts :: "set rejects an unrecognized key"` — the `config set` surface the criterion names — in addition to the schema-level property.                                                                                                                                                                                                                                      |
| FND-004 | Fixed, with a caveat stated in the artifact itself. `plan/PLAN-001-spec-correctness/` now exists so the plan-completion gate can run, and its log records that it was **authored retroactively**: it documents what was built, not what was planned.                                                                                                                                                                   |

**FND-005 is recorded, not fixed.** It surfaced while verifying the FND-002 fix
and is pre-existing: `quire validate 'spec/**/*.md'` already exited non-zero on
the tree before any of this work, for the same two reasons. The `TestMatrix`
archetype requires a `## Functional Requirement Coverage` table
(`Functional Req | Acceptance Criteria | Test Cases | Coverage Status`) and a
`## Test Case Summary` table keyed on `TC-`/`IT-` ids. quoin's matrix has neither:
it uses `## Functional Requirements` with a free-text
`` `file.test.ts` :: "test name" `` column, and no test-case table at all.

Closing it means restructuring the matrix and allocating a `TC-` id per test —
a `spec-matrix` job in its own right, with a real risk of losing the per-test
detail the current prose carries. It was left alone deliberately rather than
half-done: reshaping 28 rows into four fixed columns at the end of an unrelated
change is how that detail gets dropped. **`spec/matrix.md` gained no new failure
from this work** — the `stakeholder_coverage` table added for FND-002 was
corrected to the archetype's declared columns and validates.

The FND-001 fix was deliberately made on the implementation side. The criterion
describes behavior a user benefits from — an unknown command that names the
remedy, which is also what NFR-003 asks for — so the honest repair was to restore
the behavior the FR-026 migration dropped, not to reword the criterion down to
what the code happened to do.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                                                  | Refs                                                                  |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| FND-001 | high     | FR-005-AC-1 requires an unknown-command error "that includes usage"; the runner emits `command <x> not found` with no usage, and no test asserts usage                                                                                                   | FR-005-AC-1; FR-005 Behavior; FR-026; tests/props/fr-005.prop.test.ts |
| FND-002 | medium   | StR-001…StR-006 have no rows in `spec/matrix.md`; six stakeholder requirements with validation criteria sit outside the matrix entirely                                                                                                                  | StR-001..006; TM-001                                                  |
| FND-003 | low      | FR-027-AC-6 is verified against `QuoinConfigSchema` directly rather than through a `config set` invocation; the schema is the mechanism, not the surface                                                                                                 | FR-027-AC-6; tests/props/second-pass.prop.test.ts                     |
| FND-004 | low      | This work has no `plan/` bundle, so Step 1 could not run; the FR-first route bypassed the plan gate                                                                                                                                                      | FR-028; US-011                                                        |
| FND-005 | medium   | `spec/matrix.md` and `spec/evals.md` both fail `TestMatrix` structural validation: neither declares the required `Functional Requirement Coverage` or `Test Case Summary` sections, so `quire validate 'spec/**/*.md'` exits non-zero for the whole repo | TM-001; TM-002; spec-artifacts-process TestMatrix archetype           |

## Coverage

- **Requirements checked**: 35 (FR-001…FR-028, NFR-001…NFR-009, StR-001…StR-006).
- **Criteria reconcilable by tracking tag**: 109 of 130, from 0 before this work.
  115 `// Trace:` comments across 15 test files.
- **Criteria verified by another method**: 21 — 2 delegated to `@oclif/core`,
  12 by eval/inspection (FR-028), 1 accepted limitation, 6 StR demonstration.
- **Generated property tests**: 28 (17 unattended + 11 second-pass, accepted).
- **Suite**: 219 tests pass; the 100% v8 coverage gate holds on all four axes.
- **Semantic review**: run, not skipped. Spot-checked every criterion carrying a
  generated test; one divergence found (FND-001).
- **Step 1**: not applicable — no plan bundle (FND-004).

Counts use a right-boundary match. A naive substring check reports 109 as 110,
because `FR-025-AC-1` is a prefix of `FR-025-AC-10`.
