---
id: SR-019
title: "Gap analysis — the tier-1 runner and its ratchet (quoin#199)"
type: SpecReview
analysis: gap-analysis
scope: "scripts/bench-tier1.mjs, bench/, tests/bench-tier1.test.ts, spec/matrix.md"
review_set: subset
---

# SR-019: Gap analysis — the tier-1 runner and its ratchet (quoin#199)

## Summary

Post-implementation gate over `feat/199-tier1-runner`. Steps 2 and 3 ran in full;
step 1 has no target (`#199` is a ticket, not a plan bundle); a targeted semantic
pass ran over the eight requirement↔test↔code triples the diff adds. All eight
new Test Cases are backed by the engine's own reconciliation, and the gate itself
was mutation-verified end to end rather than assumed.

## Verdict

**CONDITIONAL** — no `high` findings. FND-001 is a real scoring hole with a named
owner and a declared state in the mapping table, not a silent one.

## Findings

| ID      | Severity | Summary                                                                                                    | Refs                       |
| ------- | -------- | ---------------------------------------------------------------------------------------------------------- | -------------------------- |
| FND-001 | medium   | `mocked-confirmation` is mapped to `audit.findings` and the runner never calls `quoin evidence audit`      | scripts/bench-tier1.mjs:79 |
| FND-002 | medium   | `cost_per_confirmed_insight` is declared and still uncomputed — the runner records no tokens or tool calls | bench/metrics.json         |
| FND-003 | low      | `span_grounding_rate` remains uncomputed; quoin#219 is unblocked by quire-cli#63 but not yet done          | bench/metrics.json         |
| FND-004 | low      | The gate is not wired into `make lint`/`make test`, so it runs only when invoked                           | Makefile                   |

## Detail

### FND-001 — a mapped family the runner cannot reach

`bench/tier1-mapping.json` maps `mocked-confirmation` to `audit.findings`, and
`findingsFor` calls `quire coverage` and `quire validate` only. So the family
scores recall 0 for a reason the table names and the runner does not act on.

This is deliberate and bounded, not an oversight: the detector **cannot fire at
all** (`agent-ix/quoin#204`, reopened — `AuditInput.injections` is never supplied
by any caller and nothing produces a `MockInjection`), so wiring the third command
today would add a subprocess per corpus and change no number. The mapping declares
the intended source so the wiring has a contract to satisfy when #204 lands.

The gap is recorded rather than hidden, which is the standard this programme
holds everything else to.

### FND-002 / FND-003 — two metrics still declared and uncomputed

`cost_per_confirmed_insight` needs a token and tool-call count per run;
`scoreCost` exists and the runner passes it nothing, because a `make` target
invoking two binaries has no agent-harness accounting to read. `span_grounding_rate`
needs a runner over `quire properties --json` — that is `quoin#219`, unblocked by
the quire-cli#63 scope fix and not yet done.

Both carry a `baseline_note` saying so. A declared-and-uncomputed metric is
honest; a declared metric silently reported as 0 is the failure the dictionary
exists to prevent.

### FND-004 — the gate runs when asked

`make bench-tier1` is not part of `make lint` or `make test`. That is the right
default for now — it shells out to a `quire` binary whose version is a variable,
and a gate that fails on a clean checkout because the host's CLI lags gets
disabled within a week. It belongs in the release gauntlet once the binary is
pinned; recorded so the choice is visible.

## Coverage

**Step 1 — plan completion.** Not applicable: `#199` is a ticket.

**Step 2 — matrix verification.** `quire coverage --scope .` with `quire` built
from `quire-rs@8c4928a`: 332/686 backed, and **none** of TC-953..TC-960 appears in
`unbacked_rows`. Every one is bound by the engine's reconciliation, not by a grep.

**Step 3 — underspecified code.** Every exported symbol the diff adds has an
owning criterion and a test: `flattenLabels` → FR-043-AC-7 (TC-953),
`localisationRate` → FR-043-AC-4 (TC-954), `compare` → FR-043-AC-10 and
FR-043-AC-6 (TC-955, TC-956, TC-957), `ratchet` → FR-043-AC-10 (TC-960). The
mapping table is data and is covered by TC-958 and TC-959. `findingsFor` and
`render` are internal and covered indirectly by the mutation run below.

**Step 4 — semantic review.** Run, scoped to the eight new triples. It is what
produced FND-001: TC-958 asserts every declared family is _mapped_, which is
weaker than every declared family being _reachable_, and `mocked-confirmation`
sits in exactly that gap. The other seven agree with their criteria's intent.

**The gate was verified, not assumed.** Retiring one mapping key — simulating a
detector that stops emitting its signal — moved `vacuous-under-guard` recall
1.00 → 0.00, printed `!!`, held the baseline at 1, and exited non-zero. Restoring
the key returned the run to green. That is the property FR-043-AC-10 asks for,
demonstrated rather than described.
