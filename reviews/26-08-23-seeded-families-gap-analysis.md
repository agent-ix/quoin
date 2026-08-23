---
id: SR-017
title: "Gap analysis — seeded families and the family cross-check (quoin#199)"
type: SpecReview
analysis: gap-analysis
scope: "evals/fixtures/bench/, evals/lib/, tests/bench-corpora.test.ts, tests/eval-quality.test.ts, spec/matrix.md"
review_set: subset
---

# SR-017: Gap analysis — seeded families and the family cross-check (quoin#199)

## Summary

Post-implementation gate over `feat/199-seed-missing-families`. `quoin#199` is a
ticket, not a plan bundle, so step 1 (plan completion) has no target and is
recorded as not applicable; steps 2 and 3 ran in full and a targeted semantic
pass ran over the five requirement↔test↔code triples the diff touches. One gap
was found and closed inside this branch: the collateral scoring path — the logic
that removes a finding from the precision denominator — had no behavioural test.

## Verdict

**CONDITIONAL** — no `high` findings survive. FND-001 was `high` when found and
is fixed and covered by TC-952; the rest are pre-existing census, unchanged by
this diff and owned elsewhere.

## Findings

| ID      | Severity | Summary                                                                                                                                       | Refs                               |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| FND-001 | high     | Collateral scoring had no behavioural test — TC-950 asserts the declaration's shape, not that it changes a score. Fixed by TC-952             | tests/eval-quality.test.ts:91      |
| FND-002 | medium   | `mocked-confirmation`'s corpus cannot be scored by any runner: the family needs `quoin evidence audit`, which the tier-1 design does not call | evals/fixtures/bench/build.mjs:352 |
| FND-003 | low      | 103 unbacked matrix rows across the repo, unchanged by this diff — pre-existing census, ~98% real per the prior sweep                         | spec/matrix.md                     |
| FND-004 | low      | One untracked test symbol (`NFR-008` in `tests/catalog.test.ts`), pre-existing and unrelated to this diff                                     | tests/catalog.test.ts              |

## Detail

### FND-001 — the tested thing was the declaration, not the behaviour

TC-950 asserts that every collateral declaration names a family, a reason and a
note. It never calls `scoreFindings`. So the logic that decides whether a finding
counts against precision — the whole point of the field — shipped in the first
draft with its shape checked and its effect untested.

This is the same class the benchmark exists to measure, and the second instance in
one branch: SR-016 FND-001 records that the identical code laundered duplicates
until self-review caught it, and no test would have. TC-952 now pins three
distinct behaviours, each of which fails independently:

- the seeded defect scores and the consequence is set aside, with
  `marker-form-mismatch` getting **no row at all** — a corpus that does not seed a
  family makes no claim about it;
- the declaration is **spent once**, so a second identical consequence is a false
  positive (this is the assertion that fails against the pre-fix code);
- a declaration whose **reason does not match** absorbs nothing, so `family`
  alone can never become a blanket excuse.

### FND-002 — a corpus with no path to its detector

The `mocked-confirmation` corpus is seeded and labelled, and no runner can score
it. The family's detector lives in quoin's auditor and is reached through
`quoin evidence audit`; `quire coverage` and `quire properties` — the two commands
the tier-1 runner design calls — cannot see it. Scoring it additionally needs an
evidence store the corpus deliberately does not carry, because a hand-written
`statementHashAtBinding` would fabricate the one field the store's suspect-link
check exists to compare.

And beneath that, `agent-ix/quoin#204` (reopened): the detector cannot fire at all,
because `AuditInput.injections` is never supplied by any caller and nothing in the
tree produces a `MockInjection`.

Both are recorded in the label's `note` and `needs_engine`, so the family's recall
of 0 states which of the two it is. This is a gap in the **runner**, which is the
next piece of work, not a gap in this diff.

### FND-003 / FND-004 — pre-existing census

`quire coverage --scope .` reports 324/678 backed, 103 unbacked rows, 0 status
lies, 1 untracked symbol, 0 diagnostics, and a TypeScript binding census of
400/564 bound. Every figure except the four rows this diff adds is unchanged. The
prior sweep sampled the 103 and split them 101 real / 2 rule, so this is a backlog
with a known composition rather than an unread number.

## Coverage

**Step 1 — plan completion.** Not applicable: `#199` is a ticket. The only plan
bundle in the tree, `plan/PLAN-001-spec-correctness`, is unrelated to this work
and was not assessed.

**Step 2 — matrix verification.** Run with `quire` built from `quire-rs@8c4928a`.
All five test cases the diff declares — TC-948, TC-949, TC-950, TC-951, TC-952 —
are backed by real tests carrying matching tracking tags. None appears in
`unbacked_rows`. The binding is the engine's reconciliation, not a grep.

**Step 3 — underspecified code.** Every symbol the diff adds has an owning
criterion: `crossCheckFamilies` → FR-043-AC-2 (TC-948, TC-949); the collateral
pass in `scoreFindings` → FR-043-AC-2 and FR-043-AC-7 (TC-950, TC-952); the four
corpora → FR-043-AC-7 (TC-951, TC-932). No orphan code, no stub.

**Step 4 — semantic review.** Run, scoped to the five triples this diff touches
rather than fanned out over the repo. It is what produced FND-001: TC-950 traces
to FR-043-AC-2 and validates the criterion's _shape_ while the criterion is about
a _score_. The other four agree with their criteria's intent and exercise real
code paths — TC-948 runs the real cross-check against the real dictionary and the
real `CORPORA`, and TC-949 mutates both directions separately so neither can mask
the other.
