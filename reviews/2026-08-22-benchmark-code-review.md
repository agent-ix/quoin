---
id: SR-014
title: "Code review — the benchmark, skeptic and evidence work (quoin#197)"
type: SpecReview
analysis: code-review
scope: "src/auditor/audit.ts, evals/lib/quality.mjs, evals/fixtures/bench/build.mjs, scripts/battletest.mjs, bench/, spec/evidence/, tests/"
review_set: subset
---

# SR-014: Code review — the benchmark, skeptic and evidence work (quoin#197)

## Summary

Reviewed the seven PRs this repository landed for `#197` (#209 through #215)
plus the two housekeeping ones (#207, #208). Gates are green — 564 tests across
51 files, prettier and eslint clean, `make validate` with zero structural
failures, `make evidence-audit` clean against its baseline. Two defects survive
them, both in the scoring code that the benchmark's own credibility rests on.

## Verdict

**CONDITIONAL** — no `high` findings. Both `medium`s are in metric arithmetic
rather than in a gate, but one of them can overstate the precision figure this
programme exists to make trustworthy.

## Findings

| ID      | Severity | Summary                                                                                                                         | Refs                           |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| FND-001 | medium   | `scoreFindings` matches by family alone and ignores `location`, so a right-family wrong-place finding scores as a true positive | evals/lib/quality.mjs:38       |
| FND-002 | medium   | An answer-key entry with `expect_metric` and no `expect_value` becomes a permanent silent miss, and nothing guards it           | scripts/battletest.mjs:78      |
| FND-003 | low      | `test_the_report_is_deterministic_and_carries_provenance` cannot fail against the current implementation                        | scripts/tests/test_bench.py:73 |

## Detail

### FND-001 — precision can be overstated (medium)

`scoreFindings` pairs a reported finding to a label on `family` only:

```js
const hit = expected.find(
  (l) => !matched.has(l.id) && l.family === finding.family,
);
```

`labels.json` carries a `location` for every seeded defect, and it is never
read. Any finding of the right _kind_ consumes a label of that kind regardless
of where it points.

**Failure scenario.** A tier-1 corpus seeds two `marker-form-mismatch` defects,
at `src/lib.rs:5` and `src/other.rs:40`. A tool reports two marker-mismatch
findings, both at `src/lib.rs:5` — one correct, one spurious duplicate. Both
match, both count as true positives: **precision 1.00 where the truth is 0.50**,
and recall 1.00 where the truth is 0.50.

That is the metric `FR-043-AC-2` exists to make trustworthy, reporting a
toolchain as more precise than it is — the same class of defect as the coverage
figure that started this programme.

Today no shipped corpus seeds two defects of one family, so no live number is
wrong. It becomes wrong the first time one does, silently.

**Fix.** Prefer a location match when both sides carry one, and fall back to
family only when the finding has no locus — recording which rule matched, so
the score can say how much of it was positional.

### FND-002 — a malformed answer-key entry is a silent permanent miss (medium)

```js
const hit = metric && Number(metric.value) === Number(finding.expect_value);
```

With `expect_value` absent, `Number(undefined)` is `NaN`, every comparison is
false, and the finding is scored **missed** forever rather than reported as a
malformed key entry.

This is not hypothetical: `AK-003` shipped in exactly that state and was caught
only because a test happened to assert its detection. The guard that would have
caught it does not exist — `tests/bench-corpora.test.ts` asserts every finding
has `id`, `family`, `summary`, `measured` and `found_by`, and asserts the
`detectable_since`/`tracked_by` rule, but never that an entry declaring
`expect_metric` also declares `expect_value`.

**Failure scenario.** Someone adds `AK-008` with `expect_metric` and forgets
`expect_value`. Recall drops by one finding, permanently, and reads as a
toolchain regression rather than a typo.

**Fix.** Assert the pairing in `bench-corpora.test.ts` alongside the existing
"exactly one expectation kind" check, and make `scoreAgainstKey` throw on an
`expect_metric` with no `expect_value` rather than scoring it.

### FND-003 — a determinism test that cannot fail (low)

```python
first = score(MANIFEST, observed, {})
second = score(MANIFEST, observed, {})
assert first == second
```

`score` is pure over its arguments — no clock, no randomness, no iteration over
an unordered map. Nothing that could be changed inside it makes this assertion
fail short of deliberately injecting non-determinism.

`rust-review` §4 asks of every assertion: _what change to the source makes this
fail?_ Here the honest answer is "adding a timestamp", which is the guard's
stated purpose and is worth keeping — but as written it is a tripwire, not a
test, and should say so.

**Fix.** Keep it, and rename it to what it guards (`the report carries no
timestamp`), asserting the absence of time-varying fields directly rather than
inferring it from two equal calls.

## Checks that passed

- **Completeness** — no `TODO`/`FIXME`/stub returns in the new modules; no
  mock-only or import-only tests; no `vi.mock` in any of the four new test
  files, so every one exercises real code.
- **Mock boundary** — the `mocked-confirmation` check takes injections as
  caller-supplied input rather than reaching into source, and treats absent
  data as _"nobody looked"_ rather than a clean bill. That is the right seam.
- **Metric shape** — `scoreCost` reads `metrics.tokenUsage.total` and
  `metrics.toolCalls`; both match what `evals/lib/metrics.mjs` actually
  returns. Verified against the emitting code, not assumed.
- **Null discipline** — `ratio()`, `scoreActionability` and `scoreCost` return
  `null` rather than `0` where there is no denominator, and
  `scoreAgainstKey` does the same for recall. `0/0` is not `0%`.
- **Registry purge** — every edited `workflow-assets/dist/index.js` passes
  `node --check`; these are vendored build artifacts where a bad edit fails at
  load rather than in the suite.
- **Gates** — `make test` 564 passed / 51 files; `make lint` clean;
  `make validate` 0 structural failures; `make evidence-audit` clean against
  the committed baseline.

## Scope note

`spec-artifacts-process`, `quire-cli` and `ix-cli-core` were reviewed in the
same pass with gates green (88 passed + 1 xfail, 164 passed, 245 passed + 1
skipped) and produced no findings. The widened `Traces To` pattern was checked
for catastrophic backtracking against a 391-character adversarial input and is
linear.

`quire-rs` findings are recorded separately in that repository's SR-053.
