---
id: SR-018
title: "Code review — the tier-1 runner and its ratchet (quoin#199)"
type: SpecReview
analysis: code-review
scope: "scripts/bench-tier1.mjs, bench/tier1-mapping.json, bench/metrics.json, evals/fixtures/bench/build.mjs, tests/bench-tier1.test.ts, Makefile"
review_set: subset
---

# SR-018: Code review — the tier-1 runner and its ratchet (quoin#199)

## Summary

Reviewed the working diff on `feat/199-tier1-runner`: `scripts/bench-tier1.mjs`,
the committed payload→family mapping, the `make bench-tier1` gate, two corpus
repairs the first run exposed, and eight tracked tests. `make lint`, `make test`
(53 files) and `make bench-tier1` are green, and the gate was mutation-verified:
retiring one mapping key takes `vacuous-under-guard` recall 1.00 → 0.00, prints
`!!`, holds the baseline at 1 and exits non-zero.

The tier-1 corpora have a production caller for the first time. Every metric in
`bench/metrics.json` that depended on them carried `baseline: null` with the note
"No tier-1 run has been scored against a toolchain yet." That note is now false,
and the diff replaces it with numbers.

## Verdict

**CONDITIONAL** — no `high` findings survive. Three defects were found by
_running_ the thing rather than reading it, and all three are fixed in this
branch.

## Findings

| ID      | Severity | Summary                                                                                                          | Refs                               |
| ------- | -------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| FND-001 | high     | `catch-all-universal` scored precision 11% because all nine corpora carried the defect, control included — fixed | evals/fixtures/bench/build.mjs:92  |
| FND-002 | medium   | `hollow-denominator` scored precision 50%: collateral was declared one-way for a symmetric consequence — fixed   | evals/fixtures/bench/build.mjs:143 |
| FND-003 | medium   | A family present in the baseline and absent from a run was silently no-news — fixed, and now a regression        | scripts/bench-tier1.mjs:262        |
| FND-004 | low      | `validate` has no JSON payload; path, line and reason are parsed out of a message string                         | bench/tier1-mapping.json:12        |

## Detail

### FND-001 — the control carried the defect

The first scored run put `catch-all-universal` at **1 true positive and 8 false
positives**. Reading the eight showed they were not the tool being wrong: every
corpus's only acceptance criterion was `universal`-shaped, so
`coverage.specific_shaped` read 0 across all nine — including `clean-control`.

The signal was a **constant**. A check that cannot stay silent on healthy input
is not a check, and a control that carries the defect is not a control; that is
the rule `clean-control` exists to enforce, and the corpus set was violating it
one level down. Every corpus but `catch-all-properties` now carries a criterion
the FR-052 classifier shapes as `idempotence`, and precision reads 100%.

This is the finding the whole programme is about, found by its own benchmark on
its first run, in the benchmark's own fixtures.

### FND-002 — collateral has to be symmetric

`hollow-denominator` scored 1 TP and 1 FP. The false positive came from
`marker-mismatch`: an unread marker makes `coverage.backed` a ratio over an
unread population, so it fires `hollow-denominator` too.

`hollow-metric` declared `no-symbol-bound` as collateral; `marker-mismatch` did
not declare the reverse. A symmetric consequence declared one way gets scored as
an error against whichever corpus forgot, so both now declare it.

### FND-003 — a deleted corpus read as a clean run

`ratchet` iterated the CURRENT report's families. A family in the baseline and
absent from the run — corpus deleted, mapping dropped, label removed — produced
no verdict at all, so the row simply vanished and the gate passed.

An absent row is indistinguishable from an absence of news, which is the same
shape as a check that cannot fail. A vanished family is now a `regressed`
verdict carrying `observed: null`, the reason why, and the baseline held so
`--update` cannot ratify the deletion. TC-960 covers it through the real
`ratchet`.

### FND-004 — the parse is the weak seam, and it is declared

`quire validate` has no `--json` payload. `--diagnostics-format json` emits
`{kind, message, severity}` with the path, the line and the reason embedded in
`message`, so the runner extracts them with a regex. A message-format change
would silently stop every `validate`-sourced family scoring.

Not silently, here: a `ValidationError` whose message looks like a finding
(`<path>: line N:`) and cannot be parsed **throws**, rather than being skipped.
The fragility is declared in `bench/tier1-mapping.json` under `parse_fragility`
and tracked as `agent-ix/quire-cli#65`.

## First scored tier-1 run

`quire` built from `quire-rs@8c4928a`, 9 corpora, 7 findings mapped:

| family                    | TP  | FP  | miss | precision | recall |
| ------------------------- | --- | --- | ---- | --------- | ------ |
| `catch-all-universal`     | 1   | 0   | 0    | 100%      | 100%   |
| `gate-that-gates-nothing` | 0   | 0   | 1    | n/a       | **0%** |
| `hollow-denominator`      | 1   | 0   | 0    | 100%      | 100%   |
| `marker-form-mismatch`    | 1   | 0   | 0    | 100%      | 100%   |
| `mocked-confirmation`     | 0   | 0   | 1    | n/a       | **0%** |
| `oracle-is-code-copy`     | 0   | 0   | 1    | n/a       | **0%** |
| `undeclared-type-value`   | 1   | 0   | 0    | 100%      | 100%   |
| `vacuous-under-guard`     | 1   | 0   | 0    | 100%      | 100%   |

**`finding_localisation_rate` 40%** — 2 of 5 true positives named where. The
three that did not: `hollow-denominator` (the diagnostic names a metric, not a
file), `catch-all-universal` (an aggregate, not a located finding), and
`marker-form-mismatch` (`no-symbol-bound` names the language `rust`, not the
marker's line). All three are quoin#197 workstream 4, and this is the number
that measures whether that work lands.

`actionability_rate` reads 29% against tier 2's 3.02% — a small clean corpus
flatters it, which is why both tiers are kept.

## Deliberate choices

- **The mapping is data, not code.** `bench/tier1-mapping.json` is the contract
  between what a tool emits and what the benchmark calls a finding, and it is
  reviewable in a diff. TC-958 enforces that every declared family is mapped and
  no mapping names an undeclared one.
- **`source: none` is a declared hole.** `gate-that-gates-nothing` maps to
  nothing because nothing detects it. Omitting it would read as an oversight;
  declaring it makes the recall of 0 a fact the file states. TC-959 requires a
  note on any such entry.
- **The ratchet is quire-rs `bench.py`'s, not a new one.** Same one-way compare,
  same `gate-zero` handling, same discipline of omitting an unreadable metric
  rather than reporting 0. Two ratchets with two behaviours is one ratchet
  nobody trusts.
