---
id: SR-016
title: "Code review — seeding the four unmeasured families (quoin#199)"
type: SpecReview
analysis: code-review
scope: "evals/fixtures/bench/build.mjs, evals/lib/quality.mjs, evals/lib/dictionary.mjs, tests/bench-corpora.test.ts, spec/matrix.md"
review_set: subset
---

# SR-016: Code review — seeding the four unmeasured families (quoin#199)

## Summary

Reviewed the working diff on `feat/199-seed-missing-families`: four new tier-1
corpora, the `wrong-type-cell` label repair, declared collateral in
`scoreFindings`, the dictionary/corpus family cross-check, and four tracked
tests. Gates are green — 52 test files, prettier and eslint clean, `quire
validate` over `spec/**` with zero `[assert]` and zero `[missing]`. Every label
in the diff was verified by running the real engine over the rendered corpus
rather than by reading the code, which is what caught FND-004 and FND-005.

## Verdict

**CONDITIONAL** — no `high` findings survive. Three defects were found in this
diff during self-review and fixed inside it; they are recorded below because a
defect found and fixed silently teaches nobody, and two of them are the same
class the diff exists to make measurable.

## Findings

| ID      | Severity | Summary                                                                                                                           | Refs                               |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| FND-001 | medium   | Collateral absorbed EVERY matching finding, so duplicates of one consequence vanished from the precision denominator — fixed      | evals/lib/quality.mjs:70           |
| FND-002 | medium   | New tests were numbered TC-1006..1009 against quoin's sequence, whose maximum is TC-947 — fixed to TC-948..951                    | tests/bench-corpora.test.ts:98     |
| FND-003 | medium   | `mocked-confirmation` was labelled `src/lib.rs:18`, a blank line, for a finding that carries no locus at all — fixed to bare path | evals/fixtures/bench/build.mjs:352 |
| FND-004 | low      | `hollow-denominator` cannot be isolated from `marker-form-mismatch` by any corpus; recorded as declared collateral                | evals/fixtures/bench/build.mjs:246 |
| FND-005 | low      | `scoreActionability` still counts collateral findings while `scoreFindings` does not; deliberate, and now stated                  | evals/lib/quality.mjs:89           |

## Detail

### FND-001 — collateral was a laundering channel

The first draft set aside every finding matching a collateral declaration. One
`no-symbol-bound` declaration would therefore absorb five `no-symbol-bound`
findings, and a toolchain reporting the same consequence five times would score
identically to one reporting it once — with four duplicates gone from the
precision denominator.

That is precisely the laundering CR-098's positional pairing was added to stop,
reintroduced through a side door, in the code whose purpose is to make precision
trustworthy. A declaration is now consumed like a label: one finding each.

### FND-002 — TC ids from the wrong repository's sequence

quoin's highest allocated TC across `main` and every remote branch is **947**.
The new tests were numbered 1006..1009 — quire-rs's neighbourhood, carried over
from the adjacent fix in that repo. It would not have collided, but it forks the
sequence and leaves 948..1005 permanently ambiguous. Renumbered to 948..951 and
added to `spec/matrix.md`, which the first draft also omitted: four tracked tests
with no owning row are four orphan tags.

### FND-003 — a label pointing at a blank line

`mocked-confirmation` was labelled `src/lib.rs:18`. Line 18 is blank; the test
symbol is on 21. But the deeper error was labelling a line at all: the auditor's
finding carries `{kind, obligation, severity, summary}` and **no locus**
(`src/auditor/audit.ts:241`), so any line is a claim about output the detector
does not produce. The label is now a bare path, which pairs on family and
correctly costs `finding_localisation_rate` — the honest reading is that this
check names an obligation, not a place to look.

### FND-004 — a family with no isolating corpus

`coverage.backed` is the only ratio metric that can go hollow:
`coverage.property_shaped` and `coverage.specific_shaped` set
`examined == matched == criteria` (quire-rs `coverage.rs:560-600`), so
`is_hollow`'s `matched: 0 if examined > 0` is unsatisfiable for them. And
`coverage.backed` goes hollow for exactly one reason — the binder read symbols
and bound none — which already emits its own `no-symbol-bound` diagnostic.

Measured, not assumed: an untagged source and a misspelled marker produce the
identical pair of diagnostics and the identical binding census. The engine cannot
separate the two causes. Declaring the collateral is the honest alternative to
scoring a correct finding as a false positive.

### FND-005 — actionability counts what precision excludes

`scoreScenario` passes the unfiltered list to `scoreActionability` and the
filtered one is internal to `scoreFindings`. This is deliberate: actionability
asks whether an emitted finding names where, and a collateral finding was still
emitted and still either does or does not. Recorded so the asymmetry is a choice
somebody can argue with rather than an accident.

## What the diff makes measurable

Before: `bench/metrics.json` declared 8 families, `CORPORA` seeded 4, and nothing
compared the two lists — so `finding_precision` and `finding_recall` were
structurally unmeasurable for half the dictionary and no test, gate or report
said so. An absent score row read exactly like a family with nothing to report.

After, with every expectation verified against `quire` built from
`quire-rs@8c4928a`:

| family                    | corpus                    | engine says                             | reads                                                   |
| ------------------------- | ------------------------- | --------------------------------------- | ------------------------------------------------------- |
| `marker-form-mismatch`    | `marker-mismatch`         | `no-symbol-bound`                       | detected                                                |
| `undeclared-type-value`   | `wrong-type-cell`         | `[assert]` at `spec/tests.md:10`        | detected, and located correctly only since quire-rs#254 |
| `catch-all-universal`     | `catch-all-properties`    | `coverage.specific_shaped` 0            | detected as a metric, not a located finding             |
| `vacuous-under-guard`     | `vacuous-property-suite`  | `vacuous-under-guard` at `src/lib.rs:7` | detected and located                                    |
| `hollow-denominator`      | `hollow-metric`           | `hollow-denominator`, no path           | detected, names no place                                |
| `oracle-is-code-copy`     | `oracle-copy`             | nothing                                 | detector has no caller (quire-rs#236)                   |
| `mocked-confirmation`     | `mocked-confirmation`     | nothing                                 | detector has no producer (quoin#204)                    |
| `gate-that-gates-nothing` | `gate-that-gates-nothing` | nothing                                 | no detector exists                                      |
| —                         | `clean-control`           | nothing                                 | the control is silent                                   |

Three families read recall 0 for three different reasons, and TC-951 requires each
label to say which. A 0 that does not distinguish "looked and missed" from "never
looked" is the failure this benchmark exists to end.

## Related work filed

- `agent-ix/quire-rs#254` — row-scoped assert findings named the separator line
  rather than the row. Fixed and merged (#255); `wrong-type-cell` is unmeasurable
  without it.
- `agent-ix/quoin#204` — **reopened**. The `mocked-confirmation` detector ships
  and cannot fire: `AuditInput.injections` is never supplied by any caller and
  nothing in the tree produces a `MockInjection`. A check that runs, evaluates one
  branch and returns empty every time is indistinguishable from a clean result —
  `gate-that-gates-nothing`, inside the tool built to detect it.
