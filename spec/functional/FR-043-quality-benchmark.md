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
| FR-043-AC-6 | The **silent-zero sentinel** is declared as a gate rather than a score: a metric emitted with `matched = 0` over a non-zero population and no accompanying diagnostic fails the run, and the declared expected value is exactly `0` with no tolerance. | Test (TC-931) |
| FR-043-AC-7 | A tier-1 corpus entry declares its seeded defects in a `labels.json` carrying, per defect, its family, its location, and whether the toolchain is expected to find it — so a scored miss is distinguishable from a defect nobody claimed was findable. | Test (TC-932) |
| FR-043-AC-8 | A tier-2 corpus entry declares a **pinned commit SHA** and an adjudicated answer key; a benchmark run against a different SHA is refused with a diagnostic naming both, never scored against the key. | Test (TC-933) |
| FR-043-AC-9 | The score report is a declared schema carrying, per metric, the enveloped value and the baseline it was compared against, and per corpus the tier and the identity it was run at. Two runs over identical inputs produce byte-identical reports. | Test (TC-934) |
| FR-043-AC-10 | Ratchet semantics: a score better than its baseline rewrites the baseline and passes; a score worse fails, naming the metric, both values, and the corpus; a score equal passes and rewrites nothing. Baseline regeneration is a deliberate act with a reviewable diff, never a side effect of a run. | Test (TC-935) |

## Dependencies

- **Upstream**: [FR-042](./FR-042-agent-eval-evidence.md) (the eval report metrics `cost_per_confirmed_insight` extends), `agent-ix/quire-rs` FR-063 (the metric provenance envelope this dictionary is the consumer-side counterpart of)
- **Downstream**: `agent-ix/quoin#199` (tier-1 corpora), `agent-ix/quoin#200` (tier-2 answer key), `agent-ix/quire-rs#231` (the engine-side benchmark gate), `agent-ix/quoin#201` (agent-eval quality dimensions), `agent-ix/quoin#203` (the committed battletest runner)

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-043-CON-1 | The benchmark scores the toolchain; it never edits it. No corpus fixture is repaired, and no check is retuned, as part of a benchmark run. | Design | Inspection of the runner: no write path into `~/dev` outside the report and baseline directories |
| FR-043-CON-2 | CI runs the benchmark on `workflow_dispatch` only. The enforcing gate is local `make`. | Process | Inspection of `.github/workflows/` — no `push` or `pull_request` trigger on the benchmark job |
| FR-043-CON-3 | A tier-2 corpus is read at its pinned SHA and never written to. | Design | Test (TC-933) — a run at a different SHA is refused rather than scored |
