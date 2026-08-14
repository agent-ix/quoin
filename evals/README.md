# quoin agent-pty eval harness

These evals measure whether the **real agent (Claude) can use quoin efficiently** —
token usage, tool-call counts, validation retries, repeated context fetches. They are
**not** unit tests (those cover mechanical correctness at 100% coverage in `src/`).
The harness drives the real `claude` agent through this CLI + the `/spec-*` skills via
[`agent-pty`](../../agent-pty) (tmux), then reads the **Claude Code session transcript**
for real metrics (`agent-pty` itself surfaces none).

Scenario definitions (TC-EV-001..TC-EV-015) live in [`spec/evals.md`](../spec/evals.md) and
are implemented as declarative data in [`scenarios/index.mjs`](scenarios/index.mjs).

## Running

```bash
make evals                    # canaries only (TC-EV-001 greenfield, TC-EV-008 repair loop)
make evals-all                # all 15 (costs tokens + ~20-40 min)
make evals MODEL=opus REPEATS=3

# or directly:
node evals/run.mjs --canary --model sonnet
node evals/run.mjs --all     --model sonnet --repeats 3
node evals/run.mjs --filter TC-EV-005 --model sonnet --keep
node evals/run.mjs --rebuild   # re-derive metrics + table from the last run's transcripts (no agent runs)
```

Every run auto-writes `evals/reports/latest.json` (full per-scenario metrics +
p50/p95 aggregates) and prints a summary table. Table columns:
`ctx` = `quoin write` packs (context fetches), `val` = `quire validate` runs,
`fail` = validations actually **rejected**, `edits` = Write+Edit. `--rebuild`
regenerates both from existing transcripts after a metrics-code change.

A selector (`--canary` | `--all` | `--filter EV-00X`) and `--model` are **required** —
`--model` so token counts are comparable across runs, and an explicit selector so a
full, token-costly run is never triggered by accident. `--repeats N` runs each
scenario N times and aggregates to p50/p95 (agent runs are non-deterministic).
`--keep` preserves scenario workdirs for debugging; `--reseed` rebuilds the module seed.

## How it works

1. **Seed (once).** `lib/seed.mjs` runs one real reconcile of the 8 default modules
   into a cached `filament/` (`evals/.seed-cache/`), then snapshot-copies it into each
   scenario's isolated `IX_HOME` so per-scenario runs are offline + fast.
2. **Per scenario.** `lib/env.mjs` makes an isolated `mkdtemp` repo + `IX_HOME` and a
   pre-generated session id (so the transcript path is known in advance). `setup(ctx)`
   seeds any fixtures.
3. **Drive.** `lib/agent.mjs` launches `claude --session-id <id> --permission-mode
bypassPermissions --model <m> --add-dir <repo>` under tmux, drives the startup
   menus, types a one-line kickoff (the full brief is dropped in `EVAL_TASK.md`), and
   waits for the completion sentinel (`<<<EVAL-COMPLETE>>>`, run by the agent as a final
   `echo`) in the transcript.
4. **Metrics.** `lib/metrics.mjs` parses the transcript JSONL: token usage
   (input/output/cache), tool calls + breakdown, and classified counts:
   `contextFetches` (= `quoin write`), `validationAttempts` (total `quire validate`
   runs) vs `validationFailures` (runs actually rejected — non-zero / "failed
   structural validation"; the real "didn't pass first try" signal, distinct from a
   validate-then-confirm habit), `flowOps`, `skillInvocations`, `edits`.
5. **Assert.** `lib/assert.mjs` independently re-checks the end state — expected files
   exist, an independent `quire validate` passes (exit code only; `DuplicateArchetype`
   stderr warnings are benign), and any flow/plugin/rejection/module-preference checks.
6. **Report.** `lib/report.mjs` writes `evals/reports/latest.json` (real metrics +
   p50/p95 aggregates) and prints a summary table.

## Requirements

- `claude` on PATH, logged in.
- `quire` >= 0.2.4 (supports `validate --scope <glob>`): built at
  `../quire-cli/target/{debug,release}/quire` or on PATH. The harness pins it for the
  agent via a shim dir.
- `agent-pty` built at `../agent-pty/dist/` (dynamically imported; **not** an npm dep,
  so the lockfile is untouched).
- `ix-flow` on PATH (for the workflow scenarios), `quoin` built (`make build`).
- Network/GitHub auth **once** to build the module seed (then offline).

`evals/` is dev-only and is **not** published in the npm package.

## Known limitations

- **TC-EV-010** (GitHub/package plugin) stands in with a versioned _local_ plugin offline;
  swap to a real `github:` source once a public plugin repo exists.
- `modelActiveMs` (transcript timestamp span) is secondary; `latencyMs` (harness
  wall-clock) is authoritative.
