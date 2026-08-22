---
id: SR-015
title: "Gap analysis — the benchmark and evidence work (quoin#197)"
type: SpecReview
analysis: gap-analysis
scope: "spec/functional/FR-043, FR-032; spec/matrix.md; bench/, evals/, scripts/battletest.mjs, src/auditor/, tests/"
review_set: subset
---

# SR-015: Gap analysis — the benchmark and evidence work (quoin#197)

## Summary

Verified this repository's half of `agent-ix/quoin#197` against the shipped
`quire v0.44.0` engine. The benchmark's own Test Matrix is the weakest artifact
the programme produced: **ten rows are marked `✅` with no backing test
anywhere in the repository**, all ten written in the session that landed the
work, and FR-043 — the benchmark specification itself — has zero tagged
verification for any of its ten acceptance criteria.

## Verdict

**FAIL** — matrix Test Cases with no backing tagged test, plus a `high`
finding. An epic titled _"measurable skeptics, not confident reporters"_
closed with its own matrix reporting confidently and measuring nothing.

## Findings

| ID      | Severity | Summary                                                                                                                             | Refs                                        |
| ------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| FND-001 | high     | TC-936 through TC-945 are marked `✅` with no test carrying those tags; all ten rows were authored this session                     | spec/matrix.md:384                          |
| FND-002 | medium   | FR-043's ten ACs cite TC-926..935, all `🚧` and untagged — the benchmark spec has no tagged verification                            | spec/matrix.md:399                          |
| FND-003 | medium   | FR-043-AC-7 requires `location` in `labels.json` but no criterion requires scoring to read it, so the field is declared and ignored | evals/lib/quality.mjs:38                    |
| FND-004 | medium   | FR-043-AC-6's silent-zero sentinel stays green only because a `quire` false positive supplies its "accompanying diagnostic"         | spec/functional/FR-043-quality-benchmark.md |
| FND-005 | low      | One untracked test and 44 matrix rows with no source symbol                                                                         | tests/catalog.test.ts:134                   |

## Detail

### FND-001 — ten rows claim a test that does not exist (high)

`quire coverage` reports ten status lies. Every one is a row this programme
added:

```
TC-936 ✅  TC-937 ✅  TC-938 ✅  TC-939 ✅  TC-940 ✅   (FR-032-AC-15, mocked-confirmation)
TC-941 ✅  TC-942 ✅  TC-943 ✅  TC-944 ✅  TC-945 ✅   (FR-043-AC-2/4/5, per-family metrics)
```

`grep -rl "TC-9xx" tests/ scripts/ evals/` returns nothing for any of the ten.
This is not a tagging convention gap — the repository does use `TC-` tags in
test bodies, as `tests/advise-command.test.ts:60` shows with
`describe("TC-150 the advisor is reachable from a command …")`.

`git blame` dates all ten to `2026-08-22`, commits `1ea06c9f` and `5eb727e3`.

**Failure scenario.** The matrix is the artifact a reader consults to ask
_"is the mocked-confirmation check verified?"_ It answers yes for five rows
where the honest answer is no. That is the precise defect class — a confident
report over an unmeasured population — that `#197` was opened to eliminate,
and it was introduced by `#197`.

**Fix.** Either tag the tests that discharge these rows, or set the status to
`🚧`. Not both directions at once: read each row and decide which is true.

### FND-002 — the benchmark spec has no tagged verification (medium)

FR-043's ten acceptance criteria each cite a Test Case:

```
FR-043-AC-1 → Test (TC-926)  …  FR-043-AC-10 → Test (TC-935)
```

All ten rows are `🚧` and none is tagged. The tests that _do_ exercise the
benchmark — `tests/bench-corpora.test.ts` and `tests/eval-quality.test.ts` —
carry tags from unrelated ranges (TC-119..132, TC-258..265) and none from
926..935.

The `🚧` status is honest, so this is `medium` rather than `high`: the matrix
does not claim otherwise. But FR-043 is workstream 2's headline deliverable,
and it shipped with every one of its criteria unverified by anything the
engine can see.

**Fix.** Tag the existing bench tests with the rows they discharge, and file
the genuine remainder.

### FND-003 — a declared field the contract never reads (medium)

FR-043-AC-7 requires each seeded defect to declare _"its family, its
**location**, and whether the toolchain is expected to find it — so a scored
miss is distinguishable from a defect nobody claimed was findable."_

`scoreFindings` pairs findings to labels on family alone:

```js
const hit = expected.find(
  (l) => !matched.has(l.id) && l.family === finding.family,
);
```

`location` is never read. The gap is not only in the code — **no acceptance
criterion requires the score to consume the field AC-7 requires the corpus to
declare.** The criterion stops one step short of its own stated intent, so a
test validating AC-7 as written passes over a location-blind scorer.

This is the semantic root of SR-014's FND-001, which recorded the code defect;
the criterion is why the code defect is not a test failure.

**Fix.** Extend AC-7 to require positional matching where both sides carry a
locus, then fix `scoreFindings` to satisfy it.

### FND-004 — the sentinel is green by accident (medium)

FR-043-AC-6 declares the silent-zero sentinel as _"a metric emitted with
`matched = 0` over a non-zero population **and no accompanying diagnostic**
fails the run."_

`quire-rs` SR-054 FND-002 establishes that `coverage.implements` is
count-shaped — its value _is_ its match count — so `matched = 0` is an honest
zero rather than a silent one, and the `hollow-denominator` the engine
currently emits for it is a false positive.

The two defects cancel. The spurious diagnostic satisfies AC-6's escape
clause, so `sentinel.silent_zero` reads `0`. **Fixing the engine defect alone
makes this sentinel fire on honest zeros**, because AC-6 shares the engine's
conflation of "matched nothing" with "read nothing".

**Fix.** Amend AC-6 to exempt count-shaped metrics in the same release that
fixes `quire-rs` FND-002. Neither change is safe alone.

### FND-005 — untracked test and unowned rows (low)

One test carries no matrix row:

```
tests/catalog.test.ts:134  aborts (strict) on a present but unparseable manifest.yaml
```

44 rows bind no source symbol. Both are pre-existing and outside this
programme's scope; recorded so the numbers below are not read as new drift.

## Coverage

`quire coverage --scope . --module spec-artifacts-process --json`, engine
`v0.44.0`:

|                            |           |
| -------------------------- | --------- |
| Rows backed                | 298 / 671 |
| Unbacked rows              | 134       |
| Status lies                | 10        |
| Untracked tests            | 1         |
| Rows with no source symbol | 44        |
| TypeScript symbols bound   | 374 / 551 |

The 549 `vacuous-under-guard` suspicions in the same report are **not** a
finding against this repository — they are an engine false positive recorded
as `quire-rs` SR-054 FND-001, where `"=> {"` matches every arrow function.

**Plan completion** — `#197` was executed ticket-first with no `plan/` bundle;
step 1 resolves against the 34 board-18 items, all `Done`.

**Semantic review** — run at the user's request over FR-043 and FR-032.
FND-003 and FND-004 are its findings: in both, a test that validates the
criterion as written passes over behaviour the criterion's own stated intent
forbids.
