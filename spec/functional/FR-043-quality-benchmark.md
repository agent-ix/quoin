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

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-043-AC-20 | `quoin validate` reports a `gate-that-gates-nothing` finding only when an explicit negative requirement claim, actual build/CI wiring and an incapable assertion identify the same shell gate. Each finding names the obligation, script, line, wiring file, failure mechanism and remedy. An unwired report, an unclaimed counter and a working gate remain silent. Findings are advisory by default and fail only under `--strict`; human and canonical JSON output describe the same findings. | Test (TC-1067..TC-1071) |

> **CR note (2026-09-29, agent-ix/quoin#654):** FR-043-AC-32 and FR-043-AC-33
> are withdrawn. They specified `make verification-relock` preparing a candidate
> verification-stack lock. That lock and its relock procedure are deleted by
> quoin#654, the `make` target does not exist, and the test the matrix cited,
> `tests/verification-relock.test.ts`, does not exist either. TC-1589..TC-1592
> are withdrawn with them. The ids are not reused.
>
> FR-043-AC-34 and FR-043-AC-35 are withdrawn for the same reason. They
> specified the lock's v2 policy (a per-module SHA-256 file inventory) and v2
> relocking and replay from it. The lock is deleted, and the matrix cited the
> same nonexistent `tests/verification-relock.test.ts` for them.
> TC-1593..TC-1596 are withdrawn with them, and so is the prose note that
> qualified AC-35. The ids are not reused.
>
> FR-043-AC-8, FR-043-AC-12, FR-043-AC-29 and FR-043-AC-31 are withdrawn. They
> specified pinned corpus and declaration SHAs, declaration digests and
> content-addressed baselines for the tier-1 and tier-2 benchmark runners. Those
> runners were `scripts/bench-tier1.mjs` and `scripts/battletest.mjs`, and the
> `scripts/` directory no longer exists, so no code or test stands behind these
> criteria. TC-933, TC-961..TC-963, TC-968..TC-970, TC-981, TC-1101..TC-1103,
> TC-1116..TC-1119, TC-1121 and TC-1122 are withdrawn with them. TC-1104 drops
> its trace to FR-043-AC-29. FR-043-CON-3 (a tier-2 corpus read at its pinned
> SHA), whose only validation was TC-933, is withdrawn with FR-043-AC-8. The ids
> are not reused.

> **CR note (2026-09-30, agent-ix/quoin#658):** FR-043-AC-1..AC-7, AC-9..AC-11, AC-13..AC-19,
> AC-21..AC-28, AC-30 and AC-36 are withdrawn. No test stands behind any of
> them: the tier-1 and tier-2 runners, the metric-dictionary loader and their
> TypeScript tests went with `scripts/`, `evals/` and the TypeScript test tree,
> and no Rust test carries these criteria. None was re-homed under another
> requirement. FR-043-AC-20 has a real test and stays. TC-926..TC-932, TC-934,
> TC-935, TC-941..TC-960, TC-964..TC-967, TC-971..TC-980, TC-982..TC-998,
> TC-1000, TC-1009, TC-1010, TC-1072..TC-1074, TC-1077..TC-1086,
> TC-1092..TC-1100, TC-1104, TC-1110, TC-1111 and TC-1120 are withdrawn with
> them. TC-1066 drops its trace to FR-043-AC-19 and keeps FR-032-AC-16. The
> tier-2 row's pinned SHA and the sentence on pinning it are removed from the
> description, and so is the CR-099 note, which recorded the commit
> revisions behind the withdrawn AC-12 and AC-13. The ids are not reused. The
> description's scored-benchmark, tier, sentinel, ratchet and CI sections and
> the CR-098 note (the metric dictionary and its loader) are removed; they
> described only the withdrawn criteria.

## Dependencies

- **Upstream**: [FR-042](./FR-042-agent-eval-evidence.md) (the eval report metrics `cost_per_confirmed_insight` extends), `agent-ix/quire-rs` FR-063 (the metric provenance envelope this dictionary is the consumer-side counterpart of)
- **Downstream**: `agent-ix/quoin#199` (tier-1 corpora), `agent-ix/quoin#200` (tier-2 answer key), `agent-ix/quire-rs#231` (the engine-side benchmark gate), `agent-ix/quoin#201` (agent-eval quality dimensions), `agent-ix/quoin#203` (the committed battletest runner)

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-043-CON-1 | The benchmark scores the toolchain; it never edits it. No corpus fixture is repaired, and no check is retuned, as part of a benchmark run. | Design | Inspection of the runner: no write path into `~/dev` outside the report and baseline directories |
| FR-043-CON-2 | CI runs the benchmark on `workflow_dispatch` only. The enforcing gate is local `make`. | Process | Inspection of `.github/workflows/` — no `push` or `pull_request` trigger on the benchmark job |
