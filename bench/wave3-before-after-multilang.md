# Wave 3 before/after, measured a second time on a corpus that can see it

**EPIC `agent-ix/quire-rs#264`.** The first run of this before/after
(`bench/wave3-before-after.md`) scored **every family `held`** and concluded
that the null result was a hole in the benchmark rather than a shortfall in the
fixes. This is that hole closed and the measurement repeated.

Two families now move, in three languages, and the reason each moves is traced
to a commit.

## What was measured

|                      |                                                                                                                              |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Runner               | `scripts/bench-tier1.mjs`, invoked as `make bench-tier1 QUIRE=<absolute path>`                                               |
| Corpus               | `agent-ix/qa-corpus` at **`42fdcaece936243772c08b1069534eb65363ca10`**, branch `epic/264-tier1-multilang` (parent `088771b`) |
| Corpus population    | **34 cases on disk, 34 scored, 0 pending.** Was 22 on disk / 21 scored / 1 pending                                           |
| Languages            | **rust 22, python 6, typescript 6.** Was rust 22, python 0, typescript 0                                                     |
| **Before** binary    | `quire 0.30.2 (engine 84740d4)`                                                                                              |
| **After** binary     | `quire 0.30.2 (engine 816e187)` — the tip of `epic/264-detection-minting-integrity`                                          |
| Engine distance      | 33 commits, `84740d4..816e187`, linear (`84740d4` is an ancestor)                                                            |
| Vendored declaration | `spec-artifacts-process` at `c197b1c`, **held constant across both legs**                                                    |

`816e187`, not the `26af2c8` the first run used: that commit was orphaned by a
squash. Both binaries were rebuilt from `agent-ix/quire-cli` sources with
`CARGO_TARGET_DIR` outside the shared `~/.cargo-target`, and passed to the
runner by absolute path — `--quire` is deliberately not a `PATH` lookup
(`scripts/bench-tier1.mjs:580-585`). The engine token in each version string above
is the runner's own `assertEngine` output.

**Before repeating the measurement, the first one was reproduced.** At engine
`816e187` on the _unchanged_ corpus at `088771b`, `--json` is byte-identical to
the run at `84740d4` and, with the `verdicts` block removed, to the baseline
committed at quoin `02219f8`. The first run's null result is confirmed at the
correct tip; what follows is a different corpus, not a different reading of the
same one.

## The result, whole run

Corpus, runner and declaration held constant; only the engine changed.

| family                    | shape    | precision before → after | recall before → after | verdict      |
| ------------------------- | -------- | ------------------------ | --------------------- | ------------ |
| `catch-all-universal`     | advisory | n/a → n/a                | 1.00 → 1.00           | held         |
| `gate-that-gates-nothing` | defect   | n/a → n/a                | 0.00 → 0.00           | held         |
| `hollow-denominator`      | defect   | 1.00 → 1.00              | 1.00 → 1.00           | held         |
| `marker-form-mismatch`    | defect   | 1.00 → 1.00              | **0.80 → 1.00**       | **improved** |
| `mocked-confirmation`     | defect   | n/a → n/a                | 0.00 → 0.00           | held         |
| `oracle-is-code-copy`     | defect   | n/a → n/a                | 0.00 → 0.00           | held         |
| `section-matches-nothing` | defect   | **n/a → 1.00**           | **0.00 → 1.00**       | **improved** |
| `undeclared-type-value`   | defect   | 1.00 → 1.00              | 1.00 → 1.00           | held         |
| `vacuous-under-guard`     | defect   | 1.00 → 1.00              | 1.00 → 1.00           | held         |

`finding_localisation_rate` **0.889 → 0.923** (8 of 9 → 12 of 13 true positives
named where). `minting.section_hit_rate` **null → 0.956** (65 of 68 declared
minting documents reached their section, over 34 cases); `null` is "this engine
does not emit the metric", never 0. Findings mapped 37 → 42.

`precision: null` reads "not measured", never zero. The four families at recall
`0.00` in both legs have no detector to score, and the mapping's own notes say
which of the three reasons applies to each.

### Per language

| family                             | rust before → after | python before → after | typescript before → after |
| ---------------------------------- | ------------------- | --------------------- | ------------------------- |
| `catch-all-universal` (recall)     | 1.00 → 1.00         | n/a → n/a             | n/a → n/a                 |
| `gate-that-gates-nothing` (recall) | 0.00 → 0.00         | no case               | no case                   |
| `hollow-denominator` (recall)      | 1.00 → 1.00         | collateral only       | collateral only           |
| `marker-form-mismatch` (recall)    | 1.00 → 1.00         | **0.50 → 1.00**       | 1.00 → 1.00               |
| `mocked-confirmation` (recall)     | 0.00 → 0.00         | no case               | no case                   |
| `oracle-is-code-copy` (recall)     | 0.00 → 0.00         | no case               | no case                   |
| `section-matches-nothing` (recall) | **0.00 → 1.00**     | **0.00 → 1.00**       | **0.00 → 1.00**           |
| `undeclared-type-value` (recall)   | 1.00 → 1.00         | no case               | no case                   |
| `vacuous-under-guard` (recall)     | 1.00 → 1.00         | no case               | no case                   |

`catch-all-universal` reads `n/a` for recall in python and typescript because
no case in those languages carries a label for it: the family is advisory and
fires on nearly every corpus, so its firings there are counted and unscored.
"no case" is written where the corpus genuinely has none, and
`corpus.yaml`'s `known_gaps.languages_not_declared` records why for each.

## What each Wave 3 fix did, per fix

| fix                                                   | axis            | effect on this corpus                                                                                                                                                                                                                                   | how established                                               |
| ----------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| quire-rs#270 — `section-matches-nothing`              | engine          | **`section-matches-nothing` recall 0.00 → 1.00 in all three languages**, and `minting.section_hit_rate` null → 0.956                                                                                                                                    | measured, both engines, 3 cases + 3 controls                  |
| quire-rs#272 — one heading name reached one heading   | engine          | no additional payload change on this corpus                                                                                                                                                                                                             | measured (no diff beyond #270's)                              |
| quire-rs#273 — suite headers are symbols              | engine          | no payload change on any case                                                                                                                                                                                                                           | measured (no diff)                                            |
| quire-rs#274 — python triple-quote state              | engine          | **`marker-form-mismatch` recall in python 0.50 → 1.00.** At `84740d4` the tagged symbol is not a candidate at all — `binding_census` is absent and the tool reports nothing; at `816e187` it is examined and `no-symbol-bound` fires at `src/lib.py:28` | measured, both engines, on `parser/triple-quote-scope-desync` |
| spec-artifacts-process#68 — TypeScript test-name form | **declaration** | **structurally invisible to this axis**: identical on both engines. Measured separately below                                                                                                                                                           | measured, both engines and both declarations                  |
| spec-artifacts-process#69 — unminted id classes       | **declaration** | comment-only in the manifest: `git diff fa56ced c197b1c` adds 284 lines of which 5 are not comments, and all 5 are #68's                                                                                                                                | measured (diff)                                               |

### quire-rs#274, the case that would have read as unchanged

`parser/triple-quote-scope-desync` reports `backed: 0` of 2 on **both** engines.
A fixture asserting only totals would have called it held. The delta is
whether the symbol exists to be examined:

|                  | engine `84740d4` | engine `816e187`                                           |
| ---------------- | ---------------- | ---------------------------------------------------------- |
| `binding_census` | **absent**       | `python: candidates 1, bound 0`                            |
| diagnostics      | none             | `no-symbol-bound` at `src/lib.py:28`, `hollow-denominator` |

Its control — a genuine docstring beside the embedded `source = """` fixture,
correctly tagged — reads `backed: 0` at `84740d4` and `backed: 1` at
`816e187`. Healthy, correctly-tagged Python read as untagged, in silence.

### spec-artifacts-process#68 is a declaration fix, and this axis cannot see it

The tier-1 runner varies the ENGINE and holds the module fixed. #68 adds a
`typescript-test-name-id` marker form to `spec-artifacts-process`'s manifest —
declaration, not code — so its verdict here is `held` by construction, not by
measurement.

Measured on the other axis, over
`cases/detection/test-name-id-in-call-title/input`:

| engine    | declaration          | `backed`  | `binding_census`     | diagnostics                             |
| --------- | -------------------- | --------- | -------------------- | --------------------------------------- |
| `816e187` | `fa56ced` (pre-#68)  | 0 / 4     | typescript 2 / **0** | `no-symbol-bound`, `hollow-denominator` |
| `816e187` | `c197b1c` (post-#68) | **2 / 4** | typescript 2 / **2** | none                                    |
| `84740d4` | `c197b1c` (post-#68) | **2 / 4** | typescript 2 / **2** | none                                    |

The third row is the control on the claim: the old engine with the new
declaration behaves identically to the new engine, so the effect is entirely
the declaration's.

**That a declaration-side fix is unscoreable by an engine-only before/after is
filed as `agent-ix/quoin#240`.** It is not a defect in #68.

## What became measurable that was not

1. **`section-matches-nothing` is a family.** It had no owning entry in
   `bench/tier1-mapping.json`, so the engine's diagnostic was read out of the
   payload and matched against nothing. Now: recall 0.00 → 1.00, precision
   1.00, in three languages (`agent-ix/quoin#236`).
2. **`minting.section_hit_rate` is declared and reported.** It appeared on
   every case at `816e187` and on none at `84740d4`, and the runner had nowhere
   to put it.
3. **The FR-065 stale-pending guard runs.** It was inert; see the mutation
   proof below.
4. **Python and TypeScript exist.** Six cases each, all bound to the ecosystem
   declaration.
5. **The score is cut by language.** A single table over a single-language
   corpus reads as a statement about the toolchain and was a statement about
   Rust.

## The mutation proof: the guard was inert, and now fires

The guard is FR-065's — "the runner SHALL fail the run when a case declaring
`pending` passes". Three mutations, each run against
`quire 0.30.2 (engine 816e187)` and each reverted immediately.

**Baseline, before the fix.** At quoin `fb8ec94`, with
`cases/minting/section-name-mismatch` marked `pending: agent-ix/quire-rs#270`
and the engine emitting that exact diagnostic, `node scripts/bench-tier1.mjs`
**exits 0** and prints only `1 case(s) excluded as pending a fix`. The fix had
landed 33 commits earlier. The check said nothing.

| mutation                                                                          | result                                                                                                                                     |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| re-add `pending: agent-ix/quire-rs#270` to the now-passing case                   | `Error: 1 pending case(s) now PASS — section-name-mismatch (agent-ix/quire-rs#270)`, exit 1                                                |
| add `id-column-matches-nothing` to an `expect.yaml`'s `diagnostic_reasons`        | `Error: … expects diagnostic reason \`id-column-matches-nothing\` that no family in bench/tier1-mapping.json claims`, exit 1               |
| mark `detection/test-name-id-in-call-title` pending (it names no `expect_reason`) | `Error: pending case … names no \`expect_reason\`, so the FR-065 stale-pending check has nothing to run and would pass it forever`, exit 1 |

`git status` in the submodule reported 0 modified files after each revert.

## Numbers that got worse

**`actionability_rate` fell from 0.10 to 0.048.** Two of 20 findings named a
row or a line at the old 21-case population; two of 42 do now. The **numerator
did not move**: not one of the 22 findings the expanded corpus added carries a
line number or a row id.

That is a real result and it points somewhere specific.
`section-matches-nothing` names the document (`spec/tests.md`) and the heading
it looked for, and carries **no line**. `no-symbol-bound` names the path and
puts the unbound symbol's `path:line` in its message text, but carries no
`line` field. So the fix for the dominant ecosystem failure mode is, by this
metric's definition, not actionable. Filed as `agent-ix/quoin#239`.

**`finding_localisation_rate` is not comparable to the old baseline's 0.857.**
The population changed. Within the expanded corpus it reads 0.889 → 0.923, and
that comparison is like-for-like.

## The baseline moved, and what moved it

`make bench-tier1-update` was run **once**, against the `816e187` leg, whose
verdict is `improved` with no `regressed` row. The `84740d4` leg is
`regressed` against the _old_ committed baseline — because the population grew
from 21 scored cases to 34, not because the engine got worse — and `--update`
was **not** run on it. The runner keeps the old baseline on a regression, which
is the property that makes that safe.

The rewritten `bench/tier1-baseline.json` now opens with what produced it:

```json
"provenance": {
  "engine": "quire 0.30.2 (engine 816e187)",
  "corpus": "42fdcaece936243772c08b1069534eb65363ca10"
}
```

The previous file recorded neither, which is why the first run had to
reconstruct its "before" engine from `quire-cli`'s pin history by timestamp and
then confirm it by rebuilding. That is `agent-ix/quoin#229`'s real defect and
this closes the report's half of it.

## A scoring defect the expansion exposed

Collateral pairing was scoped to family and reason and **not to the case that
declared it**, so one case's declaration absorbed another case's seeded true
positive. Five cases declare `hollow-denominator` collateral; the `84740d4` leg
emits exactly five `hollow-denominator` findings, one of which is the labelled
defect of `provenance/hollow-metric`. All five were consumed, and that family
read **recall 0.00 for the whole run while the per-language cut of the same run
read 1.00**.

Two contradictory numbers from one score, and a before/after built on it would
have published "`hollow-denominator` recall 0.00 → 1.00, improved" and credited
Wave 3, which does not touch that family. Filed and fixed as
`agent-ix/quoin#238`; every number in this document is post-fix.

## Reproducing this

```bash
git -C corpus rev-parse HEAD        # 42fdcaece936243772c08b1069534eb65363ca10
CARGO_TARGET_DIR=/somewhere/outside/the/shared/target \
  cargo build --manifest-path <quire-cli>/Cargo.toml --bin quire
make bench-tier1 QUIRE=/somewhere/outside/the/shared/target/debug/quire
```

For the "before" leg, build the same tree with
`quire-rs = { …, rev = "84740d4" }`. For the corpus's own grading,
`QUIRE=<binary> make verify` from the corpus root: **34/34 with 0 mismatches at
`816e187`, and 21 mismatches at `84740d4`** — every one of them inside the four
cases whose fixes are being measured.

## Exit criterion 6 is still not met

EPIC exit criterion 6 reads: "`quoin report` emits before/after/progression,
refuses deltas across unlike definitions or populations, and is byte-identical
on re-render."

**There is still no `quoin report` command.** Checked at this commit:
`quoin --help` lists `advise`, `assurance`, `catalog`, `completeness`,
`config`, `evidence`, `matrix`, `module`, `review`, `sync`, `to-plan`, `update`
and `write`. This document, like the one before it, was assembled by hand from
two `make bench-tier1` runs. `agent-ix/quoin#231` is open and unstarted.

The "refuses deltas across unlike populations" clause is not academic here: the
`84740d4` leg regresses against a baseline written over 21 cases purely because
it was scored over 34, and nothing but a human reading the two `corpora` counts
stops that from being read as an engine regression.
