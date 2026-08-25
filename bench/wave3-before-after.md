# Wave 3 before/after: the tier-1 benchmark, measured

**EPIC `agent-ix/quire-rs#264`.** This is the first before/after the tier-1
benchmark has ever been run for, and its headline is a null result: **Wave 3
moved nothing this benchmark scores, and the reason is a hole in the benchmark
rather than a shortfall in the fixes.**

## What was measured

|                           |                                                                                                       |
| ------------------------- | ----------------------------------------------------------------------------------------------------- |
| Runner                    | `scripts/bench-tier1.mjs`, invoked as `make bench-tier1 QUIRE=<path>`                                 |
| Corpus                    | `agent-ix/qa-corpus` submodule at `corpus/`, pinned at **`088771b2d1ae25403db5d7ffcf2820db1a5cfe16`** |
| Corpus population         | 22 cases on disk; 1 excluded as `pending`; **21 scored**, 20 findings mapped                          |
| **Before** binary         | `quire 0.30.2 (engine 84740d4)`                                                                       |
| **After** binary          | `quire 0.30.2 (engine 26af2c8)`                                                                       |
| Engine distance           | 37 commits, `84740d4..26af2c8`, linear (`84740d4` is an ancestor)                                     |
| Baseline compared against | `bench/tier1-baseline.json` at quoin `02219f8`                                                        |

Both binaries were built from `agent-ix/quire-cli` sources with
`CARGO_TARGET_DIR` set outside the shared `~/.cargo-target`, and passed to the
runner by absolute path. `--quire` is deliberately **not** a `PATH` lookup
(`scripts/bench-tier1.mjs:471`); the engine token in each `--version` string
above is the runner's own `assertEngine` output (`:450-460`), not a claim about
what was installed.

### Provenance of the "before"

`bench/tier1-baseline.json` records **no engine version**. Which binary produced
it had to be reconstructed: quoin `02219f8` is dated `2026-08-23 20:21:45 -0700`,
and the `quire-rs` rev pinned in `quire-cli/Cargo.toml` at that moment was
`84740d4` (set by `quire-cli@3c8bcb9`, `2026-08-23 13:37:42 -0700`; the next
repin, `b600c5e`, is `2026-08-24 00:39:02 -0700`).

That is an inference from timestamps. It was then **verified by reproduction**:
built at `84740d4`, the runner emits a report byte-identical to the committed
baseline. The reconstruction is therefore confirmed, not assumed.

## The result

Corpus and runner held constant; only the engine changed.

| family                    | shape    | precision before → after | recall before → after | verdict  |
| ------------------------- | -------- | ------------------------ | --------------------- | -------- |
| `catch-all-universal`     | advisory | n/a → n/a                | 1.0 → 1.0             | **held** |
| `gate-that-gates-nothing` | defect   | n/a → n/a                | 0.0 → 0.0             | **held** |
| `hollow-denominator`      | defect   | 1.0 → 1.0                | 1.0 → 1.0             | **held** |
| `marker-form-mismatch`    | defect   | 1.0 → 1.0                | 1.0 → 1.0             | **held** |
| `mocked-confirmation`     | defect   | n/a → n/a                | 0.0 → 0.0             | **held** |
| `oracle-is-code-copy`     | defect   | n/a → n/a                | 0.0 → 0.0             | **held** |
| `undeclared-type-value`   | defect   | 1.0 → 1.0                | 1.0 → 1.0             | **held** |
| `vacuous-under-guard`     | defect   | 1.0 → 1.0                | 1.0 → 1.0             | **held** |

`finding_localisation_rate` 0.857 → 0.857 (**held**); `actionability_rate`
0.10 → 0.10; true/false positive and miss counts unchanged in every row.

**Nothing improved. Nothing regressed. Nothing got worse.** The two `--json`
reports are byte-identical to each other and, once the `verdicts` block is
removed, to the committed baseline.

`precision: null` is "not measured", never zero. `catch-all-universal` is
declared `shape: advisory` and scores no precision at all
(`bench/tier1-mapping.json:38-39`); the four families reading `n/a` with recall
`0.0` have no detector to score — three of them say so in the mapping's own
notes, and `gate-that-gates-nothing` declares `source: none`.

## Why nothing moved

Not "the fixes did nothing". The engine's raw payload was diffed
case-by-case across all 22 cases, `84740d4` against `26af2c8`, with the
`engine` provenance block stripped. Exactly two signals changed, and neither is
one this benchmark reads.

1. **A new metric, `minting.section_hit_rate`, appears on all 22 cases** —
   absent at `84740d4`, present at `26af2c8`. It is not declared in
   `bench/metrics.json`, so the runner never looks at it.
2. **A new diagnostic, `section-matches-nothing`, fires on exactly one case** —
   `minting/section-name-mismatch`. No family in `bench/tier1-mapping.json`
   carries that key, so it maps to nothing; and that case declares
   `pending: agent-ix/quire-rs#270`, so it is excluded from scoring anyway.

Nothing else differed: no suspicion, no `binding_census`, no `totals`, and no
`quire validate` output changed on any case.

Both signals were traced to their origin by `git log -S` over
`84740d4..26af2c8` in `agent-ix/quire-rs`. Both first appear in **`a6a1144`,
`fix(coverage): the archetype matched and the declared table did not (#270)`**,
with later touches in `3d9de73` (#272) and `204dca4` (#320/#321). That
attribution is measured, not inferred.

### Attribution, per Wave 3 fix

| fix                                                   | effect on this corpus                                                                                                                                               | confidence                             |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| quire-rs#270 — section-matches-nothing                | **the only fix with an effect**: adds `minting.section_hit_rate` (all 22 cases) and `section-matches-nothing` (1 case). Scores nothing: neither signal is declared. | measured (payload diff + `git log -S`) |
| quire-rs#272 — one heading name reached one heading   | touches the same token; no additional payload change on this corpus                                                                                                 | measured (no diff beyond #270's)       |
| quire-rs#273 — suite headers are symbols              | no payload change on any case                                                                                                                                       | measured (no diff)                     |
| quire-rs#274 — python triple-quote state              | **cannot be exercised**: every one of the 22 cases declares `language: rust`                                                                                        | measured (`case.yaml` census)          |
| spec-artifacts-process#68 — TypeScript test-name form | **cannot be exercised**: no TypeScript case at this pin                                                                                                             | measured (`case.yaml` census)          |
| spec-artifacts-process#69 — unminted id classes       | no payload change on any case                                                                                                                                       | measured (no diff)                     |

The corpus at `088771b` is **100% Rust** — 22 of 22 cases. Two of the six Wave 3
fixes are for languages this pin does not contain.

## Two defects this measurement found

Neither is a ratchet regression — no family got worse, and no baseline was
lowered. Both are reasons the ratchet could not see Wave 3.

**1. The one Wave 3 signal reaching this corpus is unmapped, and the guard that
should have said so is inert** (`agent-ix/quoin#236`).
`section-matches-nothing` has no owning family.
`defectsFrom` (`scripts/bench-tier1.mjs:298-302`) derives a case's expected
defects by looking each `expect.yaml` reason up in the family table and
`continue`s past any reason it does not recognise — so `section-name-mismatch`
derives **zero** defects. The FR-065 stale-`pending` check then reads
`want.length` and returns `false` before running anything
(`scripts/bench-tier1.mjs:505-507`).

The engine **does** now emit the diagnostic; running `quire coverage` on that
case at `26af2c8` returns `"reason":"section-matches-nothing"`. The fix landed,
the `pending:` marker went stale, and the check written to catch exactly that
said nothing. This is the failure shape the runner's own header warns about: a
family that silently stops scoring reads identically to a family with nothing to
report.

**2. `make lint` had been red since `709fff5`.** 94 files failed
`prettier --check`: 93 inside the `corpus/` submodule, which was added at that
commit and never added to `.prettierignore`, and `scripts/bench-tier1.mjs`
itself, whose hand-laid-out `validate` argument array prettier rewrites. Two
commit messages in that range claim "lint clean". Both are wrong. Fixed in
`agent-ix/quoin#237`, the commit immediately before this one — which is why the
line numbers cited above are the post-format ones and do not match the file as
it stood when the measurement was taken.

## The baseline was not moved

`make bench-tier1-update` was **not** run, and `--update` was not passed on any
scoring run.

Every verdict is `held`. The runner offers `--update` only after an `improved`
verdict (`scripts/bench-tier1.mjs:698-702`), and the report is byte-identical to
the committed baseline, so an update would rewrite the file with its own
contents. There is nothing to move.

## Note on the append-vs-overwrite ordering rule

`agent-ix/quoin#229` was held to be a prerequisite for shipping Wave 3, on the
grounds that overwriting the baseline would destroy the before/after.

`scripts/bench-tier1.mjs:583` rewrites the working copy of
`bench/tier1-baseline.json` and nothing else. That file is tracked, and git
holds five revisions of it. The pre-Wave-3 values were recovered from git and
then reproduced exactly by rebuilding the engine at the pin in effect when they
were written — **without** #229, and after Wave 3 had already shipped.

The prior values were never at risk. What #229 is right about is narrower and
still real: the baseline carries **no join to the run that produced it** — no
engine token, no corpus SHA, no timestamp. That is why the "before" engine above
had to be reconstructed from `quire-cli`'s pin history by timestamp and then
confirmed by reproduction, and why a reader of the file alone cannot tell which
engine any of its five revisions describes.

## Reproducing this

```bash
git -C corpus rev-parse HEAD          # must be 088771b2d1ae...
CARGO_TARGET_DIR=/somewhere/outside/the/shared/target \
  cargo build --manifest-path ../quire-cli/Cargo.toml --bin quire
make bench-tier1 QUIRE=/somewhere/outside/the/shared/target/debug/quire
```

For the "before" leg, build the same tree with
`quire-rs = { ..., rev = "84740d4" }`.

## Exit criterion 6 is not met by this

EPIC exit criterion 6 reads: "`quoin report` emits before/after/progression,
refuses deltas across unlike definitions or populations, and is byte-identical
on re-render."

**There is no `quoin report` command.** `quoin --help` lists `advise`,
`assurance`, `catalog`, `completeness`, `config`, `evidence`, `matrix`,
`module`, `review`, `sync`, `to-plan`, `update` and `write`. This document is
the before/after the criterion is about, produced by hand from `make
bench-tier1`; the command that is supposed to emit it, refuse unlike deltas and
re-render byte-identically does not exist yet.
