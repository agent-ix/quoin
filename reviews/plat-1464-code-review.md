---
id: SR-20261112020053
title: "Code review of quoin PR 824 (PLAT-1464 widen NFR-003 and NFR-007 populations)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@4faa1137; rust/crates/quoin-cli/tests/nfr_populations/*.rs, rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs"
review_set: subset
---

# Code review of quoin PR 824

## Summary

Ticket: PLAT-1464. Scoped to `git diff origin/main...HEAD` at 4faa1137. The Rust lane
(`rust-review`, with quoin's `rust-style`) is folded in.

Run on the PR head in a fresh worktree through `locked-build.sh` (debug=0, 4 jobs, 4 test
threads): `cargo test -p quoin-cli --test tc_3003_nfr_populations` gives 12 passed, 0 failed,
1 ignored (tc_3301, needs a quire-rs checkout). It finished in 32.56 s; wall time was 34.15 s
and max RSS 354 MB. It printed `MP-250: 97 of 111 matched` and `MP-251: 7 of 111 matched`.
`cargo clippy -p quoin-cli --test tc_3003_nfr_populations --all-features -- -D warnings` and
`cargo fmt --check` are clean. No `sleep 3600` stub process was left after the run.

What was checked:

- **The matcher.** `judge` sets `actionable` only on a non-zero exit plus the per-class
  predicate. Every class predicate needs the cause (the token, path, type, or each required
  name) AND a next step or reason.
- **The ratchet.** `assert_reads` asserts `unactionable == known`. A newly failing id breaks
  it, and so does a fixed id still on the list. Both directions fail. Silent ids must also be
  in `known`.
- **Drift.** `verdict()` asserts that the plan's `population_set` equals the enumerated
  `examined` set. The population comes from `help_tree::walk` over the binary's `--help`, not
  from a list in the producer.
- **Deadlines.** Every spawn goes through `quoin()`, which has a 120 s deadline; past it the
  process is killed and the test panics, so a hang is never read as a refusal.
- **Hang probes.** They run on scoped threads, so the 24 NFR-007 pairs cost one 30 s
  production deadline. flow.rs:213 hard-codes `Duration::from_secs(30)` and offers no seam,
  so the 34 s runtime is the right trade. `cargo test` runs test binaries one after another,
  so this adds about 30 s to the gate.
- **Module sizes.** The largest file is ix_flow.rs at 268 lines; tc_3003 is 236 and mod.rs
  251. Each class is its own small module. Nothing is a monolith.

## Verdict

Sound, with no blocking code defect. I re-ran the unknown-flag, unknown-subcommand and
missing-argument probes by hand: all 78 matched messages contain `usage` or `--help`, or
name the required flags. No reading depends on a spurious substring today. The findings
below tighten the matcher, hermeticity and portability.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Next-step and position checks are bare substring matches that can pass vacuously: `available` is inside `unavailable`, `try ` is inside `registry `/`entry `, `line` is inside `pipeline`/`deadline`/`inline`, and REASONS has the generic `could not` | rust/crates/quoin-cli/tests/nfr_populations/failure.rs:14-21, rust/crates/quoin-cli/tests/nfr_populations/malformed_input.rs:38-42, rust/crates/quoin-cli/tests/nfr_populations/missing_path.rs:14-22 |
| FND-002 | low | NFR-007 `Evidence::NamesTool` (8 of 12 modes) only checks that stderr contains `ix-flow`. A regression that maps one mode to another mode's message still passes, although each mode has its own phrase (`>=0.3.1 is required`, `supported semantic version`, `invalid JSON`, `1 MiB output bound`, `no instance_id`) | rust/crates/quoin-cli/tests/nfr_populations/ix_flow.rs:216-219 |
| FND-003 | medium | Not hermetic for user config. Context sets IX_HOME, QUOIN_MODULE_PATHS and cwd, but not IX_CONFIG_ROOT/XDG_CONFIG_HOME/HOME. quoin reads `$XDG_CONFIG_HOME` or `$HOME/.config/ix/config.d/quoin.yaml` (write.rs:96-110, config.rs:148, flow.rs:280), so a developer's config can change a reading | rust/crates/quoin-cli/tests/nfr_populations/context.rs:123-128 |
| FND-004 | low | cfg(unix) gating regressed. WORKFLOW_ABOUT, Evidence, Mode, MODES, STUB and workflow_commands are unconditional but are used only by cfg(unix) fns, so they are dead code on Windows and `clippy --all-targets -D warnings` there would fail. On main, commands.rs gated every item. Windows CI builds bins only, so no CI break today | rust/crates/quoin-cli/tests/nfr_populations/ix_flow.rs:25-151 |
| FND-005 | low | Repeated `--help` spawns. Each of the 8 tests builds its own Context and repeats the full help walk. catalog_conflict and ix_flow then spawn `--help` again per command to read a summary line that `parse` could keep on CommandHelp | rust/crates/quoin-cli/tests/nfr_populations/catalog_conflict.rs:15-21, rust/crates/quoin-cli/tests/nfr_populations/ix_flow.rs:142-151 |
