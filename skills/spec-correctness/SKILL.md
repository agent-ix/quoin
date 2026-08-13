---
name: spec-correctness
description: Turn classified acceptance criteria into runnable property tests. Consumes
  `quire properties --json` per-criterion records, grounds each criterion's domain,
  precondition and oracle in the spec and the code, and emits property tests in the
  repo's own harness (proptest / fast-check / hypothesis) keyed on `row_id`. Emits a
  validated SpecReview recording every criterion it could not ground and why.
---

# Spec Correctness

Use this skill to **generate property tests from acceptance criteria you already wrote**.

`quire properties` classifies every binding acceptance criterion by property shape and
stops there — it names no test framework and its deterministic recall has a measured
ceiling. This skill closes the three gaps that leaves:

1. **The harness.** The shape→harness mapping is this skill's, not the engine's.
2. **The clauses.** Domain/precondition/oracle spans reach only a small minority of
   criteria, so this skill grounds them from the spec and the code itself.
3. **The residue.** Criteria the deterministic pass labelled `example` or `unclassified`
   go to an LLM second pass — review-gated, so it can afford recall the engine cannot.

Specified by [FR-028](../../spec/functional/FR-028-generate-property-tests-from-criteria.md).

## The one rule that is not negotiable

**A low extractable ratio is data, never a verdict.** This skill reports counts. It emits
no threshold, no grade, no pass/fail, and it never suggests rewording a criterion to score
better. A criterion that describes one concrete scenario is a good criterion; it just isn't
a property. (quire-rs FR-052-CON-1.)

Two more, inherited:

- **Never write a framework name into `spec/**`.** (CON-2.)
- **Never propose a new regex or lexicon entry for quire-rs.** 31.3% recall at 93.3%
  precision is a measured ceiling, not a bug. (quire-rs#45.)

## When to use

- After acceptance criteria are written and validated, to give them verification.
- Before `gap-analysis`, so the matrix rows it reconciles have real backing tests.
- On an existing repo, to find which criteria have no test and why.

Not for: authoring criteria (`specify`), building the matrix (`spec-matrix`), or judging
whether existing tests are good (`gap-analysis`).

## Inputs

- The target repo and its spec glob (default `spec/**/*.md`).
- `quire properties --scope <repo> --json '<glob>'` output — quire-cli ≥ 0.12.0.
- The repo's source and test trees.

## Steps

0. **[Scope and harness](references/step-0-scope-and-harness.md)**: resolve the repo and
   glob, run `quire properties --json`, detect the harness, test dir, and the repo's
   existing tracking-tag style.
1. **[Census](references/step-1-census.md)**: bucket records by `property` × `extraction`.
   Data only — no verdict, no threshold, no rewording suggestion.
2. **[Clause grounding](references/step-2-clause-grounding.md)**: derive domain,
   precondition and oracle for each record, each cited to a `file:line`. No citation, no
   test.
3. **[Strategy selection](references/step-3-strategy-selection.md)**: pick the assertion
   skeleton from `property`, and the routing lane from `extraction`.
4. **[Generate tests](references/step-4-generate-tests.md)**: emit into the repo's harness
   with the `row_id` tracking tags and the provenance line.
5. **[Second pass](references/step-5-second-pass.md)**: the LLM pass over `example`,
   `unclassified`, and anything step 2 refused to ground.
6. **[Review artifact](references/step-6-review-artifact.md)**: the `SpecReview` at
   `reviews/YY-MM-DD-<slug>.md` recording what could not be grounded, and why.
7. **[Report and handoff](references/step-7-report-and-handoff.md)**: the run report, the
   `Property` rows for `spec-matrix`, and the reconciliation check against `gap-analysis`.

Steps 0–4 and 6–7 always run. Step 5 is an expensive LLM pass — **ask before running it**
when the residue is large (say, more than 30 records).

## Outputs

| Path | What |
| --- | --- |
| `tests/props/<per-harness naming>` | Property tests, all of them runnable |
| `reviews/YY-MM-DD-<slug>.md` | A `SpecReview` (`analysis: spec-correctness`) recording what could not be grounded |

Nothing else is written to the tree (FR-028-CON-1). No skipped tests, no `_review/`
directory, no ad-hoc report file. A generated test arrives in a pull request, and that is
what puts it under review; the artifact carries the one thing a PR does not — why a
criterion was left ungrounded.

## Two facts that will bite if you assume otherwise

- **`shape` is not `property`.** `shape` is the FR-047 grammar axis
  (`assertion | obligation | given-when-then | unstructured`). `property` is the 10-value
  taxonomy. **Strategy selects on `property`.** Routing selects on `extraction`.
- **StR scores highest, not lowest** — 29.1% against FR's 19.2%. Do not down-weight
  stakeholder requirements (quire-rs CR-029 falsified the CR-020 prediction).
