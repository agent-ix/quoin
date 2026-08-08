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

**FAIL** — one `high` finding: an acceptance criterion whose stated oracle is
neither met by the implementation nor asserted by the test that claims it.

The failure is narrow and predates this work. Everything Phase C itself set out to
do is done: the skill exists, is specified, is verified at the eval layer, is
released, and is consumed.

## Findings

| ID      | Severity | Summary                                                                                                                                                  | Refs                                                                  |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| FND-001 | high     | FR-005-AC-1 requires an unknown-command error "that includes usage"; the runner emits `command <x> not found` with no usage, and no test asserts usage   | FR-005-AC-1; FR-005 Behavior; FR-026; tests/props/fr-005.prop.test.ts |
| FND-002 | medium   | StR-001…StR-006 have no rows in `spec/matrix.md`; six stakeholder requirements with validation criteria sit outside the matrix entirely                  | StR-001..006; TM-001                                                  |
| FND-003 | low      | FR-027-AC-6 is verified against `QuoinConfigSchema` directly rather than through a `config set` invocation; the schema is the mechanism, not the surface | FR-027-AC-6; tests/props/second-pass.prop.test.ts                     |
| FND-004 | low      | This work has no `plan/` bundle, so Step 1 could not run; the FR-first route bypassed the plan gate                                                      | FR-028; US-011                                                        |

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
