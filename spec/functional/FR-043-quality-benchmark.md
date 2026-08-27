---
id: FR-043
title: "The quality benchmark"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-042"
    type: "extends"
---

# FR-043: The quality benchmark

## Description

Battletest pass 2 against `agent-ix/filament-ide-rs` delivered one verdict about this toolchain:
**good reporters, poor skeptics.**

- `quire coverage` printed `555/2389 backed (23%)` while its declared tag patterns matched **0 of
  1,292** `fn tc_NNN_` symbols and **0 of 643** trace lines. Three published SpecReviews cited
  coverage figures that measured nothing.
- `quire properties` headlined `515/951 criteria extractable (54%)` — 440 of the 515 being the
  catch-all `universal` shape, so the honest figure for *"the tool told me what property to write"*
  was **78/951 (8%)**.
- **Every conclusion-changing finding of the pass came from manual work.** Not from tool output.

The metric-integrity half of that is fixed (quire-rs FR-063, CR-093..CR-097). What is not fixed is
that **nothing measures whether the toolchain is getting better at finding real defects.** A check
can be added, fire a thousand times, and nobody can say whether any of them were true.

quoin SHALL carry a **scored benchmark**: corpora with known answers, metrics that declare what they
count, and a ratchet that fails when a score regresses.

### This spec is the enforcement, not a description of one

The metric dictionary below is normative. A benchmark metric that does not declare its **unit**,
**population** and **method** is not a metric this benchmark emits — which is the same rule
quire-rs FR-063 applies to the engine's own numbers, applied one layer up to the thing that scores
the engine.

That ordering is deliberate. The failure being closed is a *number nobody could interrogate*;
building the harness first and describing it afterwards would reproduce it.

### Two corpus tiers, and why both

| tier | corpus | ground truth | answers |
|---|---|---|---|
| **1** | synthetic seeded-defect mini-repos | `labels.json`, hand-written alongside the defect | *does the tool find a defect we know is there* |
| **2** | `filament-ide-rs` at a **pinned SHA** | adjudicated answer key from pass 2 | *does it find the defects a human found in the wild* |

Tier 1 alone overfits: a seeded defect is one somebody already knew how to describe, so a tool tuned
to it scores well and finds nothing new. Tier 2 alone cannot isolate: a real corpus changes under
you, and a score that moves says nothing about which change moved it. **Pinning the SHA is what
makes tier 2 a measurement rather than an observation** — the same discipline that made the pass-2
findings reproducible.

### The silent-zero sentinel is a hard failure, not a score

Every other metric here is a number to improve. This one is a **gate**: a metric emitted with
`matched = 0` over a non-zero population and **no accompanying diagnostic** must be zero, always.

That is the exact shape of `555/2389 (23%)` — arithmetic over a corpus the binder could not read,
published with nothing saying so. It is not a quality to trade off against precision; it is the
class of defect that made three reviews wrong, and a benchmark that let it score 0.98 and pass would
be measuring the wrong thing.

### Ratchet, not threshold

Scores are compared against **checked-in baselines**, not against invented targets. An improvement
tightens the baseline automatically; a regression fails. A hand-picked threshold invites the
number to be tuned to it, which is how `ac:unclassifiable` came to pass 99.2% of corpus cells
(quire-rs CR-019).

The ratchet reuses the existing `quoin evidence audit --ratchet` conventions rather than inventing a
second set.

### CI stays `workflow_dispatch`-only; the gate is local `make`

Unchanged from this repository's standing posture. The benchmark is expensive — tier 2 walks a
24-crate repository — and a gate that runs on every push would be disabled within a week.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-043-AC-1 | The metric dictionary declares, for every benchmark metric, its `unit` (what one of the value is), `population` (what the denominator is drawn from), and `method` (how it was arrived at, and what a partial read means). A metric missing any of the three is rejected at load, not reported with a gap. | Test (TC-926) |
| FR-043-AC-2 | The dictionary defines `finding_precision` and `finding_recall` **per defect family**, each keyed on a family the corpora label, so a score cannot be reported over an unlabelled population. | Test (TC-927) |
| FR-043-AC-3 | The dictionary defines `span_grounding_rate` — of the criteria carrying a specific property shape, the fraction whose `domain`, `precondition` and `oracle` are all present — with the pass-2 figure (0 of 65) recorded as the baseline it starts from. | Test (TC-928) |
| FR-043-AC-4 | The dictionary defines `actionability_rate` — of emitted findings, the fraction carrying a row id — with the pass-2 figure (15 of 496) recorded as its baseline. | Test (TC-929) |
| FR-043-AC-5 | The dictionary defines `cost_per_confirmed_insight` in tokens **and** tool calls per true-positive finding, extending the FR-042 eval report metrics rather than introducing a second accounting. | Test (TC-930) |
| FR-043-AC-6 | The **silent-zero sentinel** is declared as a gate rather than a score: a **ratio-shaped** metric emitted with `matched = 0` over a non-zero population and no accompanying diagnostic fails the run, and the declared expected value is exactly `0` with no tolerance. A **count-shaped** metric is exempt — `matched` and its value are the same fact, so a zero reports that none was found, not that none was read. | Test (TC-931) |
| FR-043-AC-7 | A tier-1 corpus entry declares its seeded defects in a `labels.json` carrying, per defect, its family, its location, and whether the toolchain is expected to find it — so a scored miss is distinguishable from a defect nobody claimed was findable. The score **pairs a finding to a label by that location** where both carry one, so a right-family wrong-place finding scores as a false positive; a finding naming no place may still pair on family alone, and the report states how many pairings were positional. | Test (TC-932, TC-946) |
| FR-043-AC-8 | A tier-2 corpus entry declares a **pinned commit SHA** and an adjudicated answer key; a benchmark run against a different SHA is refused with a diagnostic naming both, never scored against the key. | Test (TC-933) |
| FR-043-AC-9 | The score report is a declared schema carrying, per metric, the enveloped value and the baseline it was compared against, and per corpus the tier and the identity it was run at. Two runs over identical inputs produce byte-identical reports. | Test (TC-934) |
| FR-043-AC-10 | Ratchet semantics: a score better than its baseline rewrites the baseline and passes; a score worse fails, naming the metric, both values, and the corpus; a score equal passes and rewrites nothing. Baseline regeneration is a deliberate act with a reviewable diff, never a side effect of a run. | Test (TC-935) |
| FR-043-AC-11 | An answer-key entry declaring `expect_metric` without a usable `expect_value` **fails the run as malformed**, never scores as a miss: `Number(undefined)` is `NaN`, so every comparison is false and the finding would read as a permanent toolchain regression rather than a typo. | Test (TC-947) |
| FR-043-AC-12 | A tier-1 report records the **declaration** it was scored against — a content digest over the module tree, a digest per module the corpora bind, and the upstream SHA the corpus records for each vendored module — on the same footing as the engine and the corpus revision. The declaration is a **run-time variable**: the same corpus can be scored with the engine held fixed and the declaration moved, which is the only way a declaration-side fix is distinguishable from a fix that had no effect. A module id resolving to no manifest under the declaration root fails the run; a `VENDORED.md` present and unparseable fails the run; a declaration root carrying none records `sources: null`. A provenance row is recognised by its module cell resolving to a directory in the same tree, and **any single such row whose SHA cell does not yield a hash fails the run** — dropping it would leave a confident `sources` naming only the modules that happened to parse. A hash followed by an annotation is read, the hash being the first token. | Test (TC-968, TC-969, TC-970, TC-981) |
| FR-043-AC-14 | The runner consumes the validated `cases` and `bounds` envelope from qa-corpus's canonical `bounds.py --json` reader. It does not merge `case.yaml` variants or interpret either on-disk layout itself, leaving exactly the intentional Python and Rust metadata readers. The envelope must carry numeric bounds and, for every case, a non-empty id, language, module, directory and expectation path; unresolved paths fail by case id. | Test (TC-974, TC-975, TC-976, TC-977, TC-978) |
| FR-043-AC-15 | A `pending:` marker's expiry signal is read from **`expect-pending.yaml`**, never from the live block — the live block states what is true today, and for a case whose defect is silence the future reason appears there only as `absent_diagnostic_reasons`, indistinguishable from reasons that must stay absent after the fix. Staleness is evaluated against the **raw payload**, so a token no family scores is still seen. A pending case with **no** forward block fails the run; a forward block this runner cannot evaluate — one stating a payload change rather than a diagnostic — is **deferred to the corpus's own graders and named on stderr**, never silently passed. | Test (TC-979, TC-980) |
| FR-043-AC-18 | A benchmark update **appends a `MeasurementRecord` to a series** and prior records survive; the single-snapshot baseline becomes a derived convenience file the ratchet reads, not the source of truth. One producer invocation is **one atomic record**, written whole and only after the run completed, so a partial run does not land. Each record carries the fields a delta needs to be honest — definition version, subject, scope including the scored population and its per-language census, tool identity, **tool version and configuration digest read from the payload envelope and never from an operator-supplied string**, corpus revision, scorer revision, units, timestamp — and retains the raw report as **attached evidence** rather than transcribed figures. The ratchet's pass/fail behaviour is unchanged. | Test (TC-997, TC-998, TC-1000) |
| FR-043-AC-17 | The benchmark is reachable from **one local `make` target** that also runs lint and the unit suite, so a human running the gate cannot skip a leg of it; CI remains `workflow_dispatch`-only and is not that target. The target names the **engine binary** it measured with, because `quire --version` reports the CLI crate version and a current CLI can link a stale engine. Separately, the **committed baseline is held to the committed scorer** by the unit suite: comparing the baseline against itself must yield `held` on every verdict and never `new`, and every family the baseline scores must be one the mapping still claims. Generated baseline JSON already satisfies the repository format gate, so the supported update workflow cannot make the next gate red without a metric change. | Test (TC-995, TC-996, TC-1077) |
| FR-043-AC-16 | An **advisory** family's precision is scored only over the corpus cases that have **ruled** on it — its reason named in a case's `diagnostic_reasons` (must fire) or `absent_diagnostic_reasons` (must stay silent) — and a ruling **scoped to one declaration** (`test-case/…`) governs only findings that declaration raised. Multiple standing entries resolving to the same advisory family are unioned by declaration; a later narrow ruling cannot erase an earlier one. Every other firing is counted as **`unadjudicated`** and published beside the rate, never folded into it: a `null` precision must state how many findings nobody has ruled on. The rate cannot fall independently of the corpus's own differential gate, so it is the `unadjudicated` **count** that is ratcheted, `lower-is-better`. A family the baseline measured that now reports a `null` precision is **`regressed`**, naming the family — reclassifying a family to `advisory` may not delete its number in silence. A family that never had one is skipped, so a detector that does not exist cannot hold the gate permanently red. | Test (TC-982, TC-983, TC-984, TC-985, TC-986, TC-987, TC-1078) |
| FR-043-AC-13 | The ratchet **refuses a delta across unlike inputs**. Two reports differing in corpus revision, declaration digest, or scored population — count or per-language mix — are `incomparable`: both values are reported, neither `improved` nor `regressed` is claimed, and the run exits non-zero so moving an input is a deliberate, reviewable re-baseline. The **engine** is deliberately not such a field — varying it and comparing is what the benchmark is for. A baseline recording nothing for one of those fields is reported as **unknown** rather than assumed to match. | Test (TC-971, TC-972, TC-973) |
| FR-043-AC-19 | Corpus loading, execution, scoring, comparison, persistence and rendering are separate acyclic modules behind the runner. Execution preserves structured diagnostic locations; human and JSON views consume the same report object; module behavior is directly tested rather than reachable only through the command. | Test (TC-1009, TC-1010) |
| FR-043-AC-20 | `quoin validate` reports a `gate-that-gates-nothing` finding only when an explicit negative requirement claim, actual build/CI wiring and an incapable assertion identify the same shell gate. Each finding names the obligation, script, line, wiring file, failure mechanism and remedy. An unwired report, an unclaimed counter and a working gate remain silent. Findings are advisory by default and fail only under `--strict`; human and canonical JSON output describe the same findings. | Test (TC-1067..TC-1071) |
| FR-043-AC-21 | Every Tier-2 answer-key entry names one production source and one signal. The battletest executes the source registry against the pinned tree. An evaluated source returning no matching signal is a miss; an unavailable premise is a source-specific `not evaluated` state naming the missing input, excluded from recall and forbidden in a baseline update. Family-level Tier-1 detection never substitutes for evaluation of the adjudicated Tier-2 finding. | Test (TC-1072..TC-1074) |
| FR-043-AC-22 | Tier 1 scores `untracked-id-has-minted-children` as its own located `unminted-id-guidance` family. It is not folded into `untracked-id-near-miss`: the former gives model-grounded guidance for a direct parent or nested undeclared class without backing the authored id, while the latter repairs two spellings of one id. | Test (TC-1079) |

## Dependencies

- **Upstream**: [FR-042](./FR-042-agent-eval-evidence.md) (the eval report metrics `cost_per_confirmed_insight` extends), `agent-ix/quire-rs` FR-063 (the metric provenance envelope this dictionary is the consumer-side counterpart of)
- **Downstream**: `agent-ix/quoin#199` (tier-1 corpora), `agent-ix/quoin#200` (tier-2 answer key), `agent-ix/quire-rs#231` (the engine-side benchmark gate), `agent-ix/quoin#201` (agent-eval quality dimensions), `agent-ix/quoin#203` (the committed battletest runner)

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-043-CON-1 | The benchmark scores the toolchain; it never edits it. No corpus fixture is repaired, and no check is retuned, as part of a benchmark run. | Design | Inspection of the runner: no write path into `~/dev` outside the report and baseline directories |
| FR-043-CON-2 | CI runs the benchmark on `workflow_dispatch` only. The enforcing gate is local `make`. | Process | Inspection of `.github/workflows/` — no `push` or `pull_request` trigger on the benchmark job |
| FR-043-CON-3 | A tier-2 corpus is read at its pinned SHA and never written to. | Design | Test (TC-933) — a run at a different SHA is refused rather than scored |

> **CR-098 note (2026-08-22):** `agent-ix/quoin#198`, `#201`, reopened. Three
> corrections to criteria that shipped, each found by review rather than by a
> failing test (SR-014, SR-015).
>
> **AC-6 was green by accident.** It excuses a metric with `matched = 0` when a
> diagnostic accompanies it — and the diagnostic doing the excusing was
> `quire`'s own false-positive `hollow-denominator`, which fired on every
> **count**-shaped metric reading an honest zero. Two defects cancelling:
> `sentinel.silent_zero` read `0` because both were wrong. AC-6 now exempts
> counts by shape, and ships with `agent-ix/quire-rs#229`. Neither half is safe
> alone.
>
> **AC-7 declared a field the score never read.** It required `labels.json` to
> carry a `location` per defect *"so a scored miss is distinguishable from a
> defect nobody claimed was findable"*, and then no criterion required the
> scorer to consume it. `scoreFindings` paired on family alone, so two findings
> of one family both scored true even when one pointed where no defect was
> seeded: **precision 1.00 where the truth is 0.50**. A test validating AC-7 as
> written passed over a location-blind scorer, because the criterion stopped one
> step short of its own stated intent.
>
> A positioned finding that matches no label at that place is now a false
> positive, and may fall back to family-only *only* against labels that name no
> place — otherwise the second pass hands it the label the first refused it,
> which is the laundering the fix exists to stop. The count of positional
> pairings is reported, because a precision figure built entirely from
> family-only matches is weaker evidence than the same figure built from
> findings that named where.
>
> **The dictionary itself did not exist.** AC-1 through AC-6 each say the
> dictionary *declares* or *defines* something, and there was no dictionary:
> `span_grounding_rate`, `actionability_rate` and `cost_per_confirmed_insight`
> appeared only in this document's prose. All ten of FR-043's criteria shipped
> with no tagged test, which is what SR-015 FND-002 recorded — the deeper truth
> being that half of them were unimplemented.
>
> `bench/metrics.json` and `evals/lib/dictionary.mjs` are that dictionary and
> its loader. An entry missing unit, population, method or direction is refused
> **at load**, not reported as a gap downstream where it would be one warning
> among many; a `gate-zero` metric carrying tolerance is refused too, because a
> gate with tolerance is a score wearing a gate's name; and a `per_family`
> metric over no labelled families is refused, which is AC-2's "cannot be
> reported over an unlabelled population" made mechanical.
>
> `span_grounding_rate` is declared with its measured 0-of-65 baseline and is
> **not computed** — no runner reads `quire properties --json` for it. Declared
> anyway, because a metric nobody declared is a metric nobody can ask for, and
> the gap is tracked as `agent-ix/quoin#219` rather than left implicit.
>
> AC-7 through AC-10 were already implemented in `scripts/battletest.mjs` and
> the corpora builder, and needed tests rather than code. TC-934 asserts the
> report carries no time-varying field rather than calling a pure function
> twice, and TC-935 pins the property a scalar recall cannot express: gained
> and LOST are named per finding, so a regression that leaves recall unchanged
> still reports which detector rotted.
>
> **AC-11 is new.** An entry declaring `expect_metric` with no `expect_value`
> scored **missed forever**: `Number(undefined)` is `NaN` and every comparison
> against it is false. `AK-003` shipped in that state and was caught only
> because a test happened to assert its detection. Malformed keys now fail the
> run.

> **CR-099 note (2026-08-24):** `agent-ix/quoin#240`. **AC-12 and AC-13 are
> new**, and they close the last of the three inputs this benchmark varies
> without saying so.
>
> **The runner varied one thing: the binary.** The traceability declaration
> every case binds was whatever the corpus vendored at its pinned SHA, and it is
> an input the toolchain's behaviour depends on as directly as the engine's:
> `spec-artifacts-process#68` is **five non-comment lines** of manifest
> (`git diff fa56ced 2ed3bb9 -- spec_artifacts_process/manifest.yaml` is +101
> −0, of which 96 are comment or blank), and those five lines decide whether a
> TypeScript test's own title can bind at all. Two of EPIC
> `agent-ix/quire-rs#264`'s six Wave 3 fixes live there, so an engine-only
> before/after reported them `held` **by construction**, in the same word the
> runner prints for a family that genuinely did not move.
>
> Measured, engine held fixed and the declaration moved between
> `spec-artifacts-process` `fa56ced` (pre-#68) and `c197b1c` (the vendored pin),
> over the same 34 cases: `hollow-denominator` precision **1.00 → 0.33** and
> `marker-form-mismatch` precision **1.00 → 0.71**, identically on engine
> `84740d4` and engine `816e187`. Exactly two of 34 cases move and both are
> TypeScript; the other 32 are byte-identical, which is the half of the result
> that says the declaration change is TypeScript-only rather than a general
> perturbation.
>
> **AC-13 is the refusal the last pass needed and did not have.** The `84740d4`
> leg of the previous before/after read `regressed` on every family because the
> scored population had gone 21 → 34, and nothing in the output said so. That is
> EPIC exit criterion 6's *"refuses deltas across unlike definitions or
> populations"* and `agent-ix/quoin#231`'s unimplemented clause. The engine is
> deliberately excluded — varying it and comparing is what the benchmark is for.
