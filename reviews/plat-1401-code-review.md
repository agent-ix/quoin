---
id: SR-20261111201425
title: "Code review of quoin PR 815 (PLAT-1401 proportion plans bound to populations)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@4fe06bc; rust/crates/quoin-measurement/src/{plans/design.rs,report/inconclusive.rs,report/verdict.rs,types/plan.rs,verify/mod.rs,verify/reason.rs,verify/rows.rs}, rust/crates/quoin-cli/tests/nfr_populations/*, rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs, rust/crates/quoin-measurement/tests/tc_3000_proportion_population.rs, fixture edits in tc_478, tc_958, tc_960, tc_961, tc_975, tc_1016, tc_1954, tc_2400, MP-961 and portfolio fixtures"
review_set: subset
---

# Code review of quoin PR 815

## Summary

Ticket: PLAT-1401. Scoped to `git diff origin/main...HEAD` at 4fe06bc (base f4ffc26, PR #811).
The Rust lane (`rust-review`, with the repository `rust-style` skill) is folded in. quoin
0.30.0 was used for `quoin matrix`. Builds ran through locked-build.sh with debug=0 and
`-j 4`, in a separate review worktree.

Results:

- `tc_3003_nfr_populations`: 3 passed.
- `tc_3000_proportion_population`: 3 passed.
- `cargo fmt --check`: clean.
- `cargo clippy -p quoin-cli -p quoin-measurement --all-targets --all-features -D warnings`: clean.
- `cargo test -p quoin-measurement -p quoin-jev`: 392 passed, 1 failed (FND-001).
- `cargo test -p quoin-cli -p quoin-core`: 428 passed, 0 failed.

Mutation check: `quoin-catalog` was changed so that a manifest which fails to parse is
skipped instead of aborting. TC-3005 then went red, with 6 of 7 corrupt kinds unmatched,
so the NFR-008-M-1 producer does fail when the behaviour regresses. The mutation was
reverted afterwards.

## Verdict

**NOT MERGEABLE: two high findings make the head red.** The new rule is small and placed
correctly. `proportion_without_population` is checked in `usable`, which the gate,
ratchet, target, vanished-slice and portfolio paths all share, and also in `assess`. It is
checked after the plan and definition mismatches. The parse refuses a blank or non-string
exemption with `QM-PLAN-INVALID`, and the reason codes are added to `all()`, to `as_str()`
and to the inconclusive verdict mapping.

The fixture edits keep what each test asserts. In tc_975_024, `take(8)` is correct for the
extra line, so the no-decision-rule case still lacks only the rule. The other fixtures only
add the exemption, which restores the behaviour they had before.

The producers spawn the built binary and order results deterministically with `BTreeSet`.
They write only to `tempfile` directories, unless the caller opts in with
`NFR_COLLECTION_OUT`. NFR-007 is gated by `cfg(unix)`. On Windows the test does not exist,
so it records nothing and nothing reads as measured.

Two problems make the producers not fully honest:

- They depend on the network (FND-003).
- The NFR-007 match does not prove `ix-flow` was reached (FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR head is red. `tc_2016_the_stored_collection_is_decided_against_a_recomputed_baseline` (quoin-jev tc_2013_graded_corpus.rs, landed by #811, the PR's base) uses a `proportion` fixture plan with no `population_set` or `population_exempt`. `verify` now returns no decision, so the test panics on `decisions[0]` (index out of bounds). Add an exemption to the fixture | rust/crates/quoin-jev/tests/tc_2013_graded_corpus.rs:215, rust/crates/quoin-jev/tests/tc_2013_graded_corpus.rs:271 |
| FND-002 | high | MP-244 (spec/assurance/MP-244-jev-criterion-strength-agreement.md, from #811) is a `proportion` plan with neither a set nor an exemption. Every report and verify run now reads it as inconclusive `population_set_undeclared`. The sweep covered MP-216..219 and MP-228..230 but missed this one. Declare a set or give it an honest exemption | spec/assurance/MP-244-jev-criterion-strength-agreement.md |
| FND-003 | high | The TC-3005 producers are not hermetic, and NFR-008-M-1 can pass for the wrong reason. Measured: `quoin catalog list` with an empty `IX_HOME` runs `modules.ensure_defaults`, which git-clones the ten default modules from GitHub. Offline it exits 1 with `QM012_GIT_TRANSPORT` and empty stdout. That satisfies the corrupt-kind predicate (non-zero exit, empty stdout) for every kind, so MP-253 reads 100%. `assert_accepted("MP-253")` runs, and writes `NFR_COLLECTION_OUT/MP-253.json`, before the missing-manifest probes fail. Fix: before probing, run a positive control in the same home where the companion alone lists, and seed or disable the default install. Require the abort to name the candidate root or a manifest-YAML error. Bound each spawn with a timeout, since a network stall hangs the test | rust/crates/quoin-cli/tests/nfr_populations/manifests.rs:60-75, rust/crates/quoin-cli/tests/nfr_populations/mod.rs:73-118 |
| FND-004 | medium | The NFR-007-M-1 match is any non-zero `quoin` exit. Nothing shows that `ix-flow` was reached, so a regression that rejects the skill directory or `--path` before launch would still score 100%. Today it is reached: the stub's stderr shows `--version --json`, and rc 7 propagates. But the failing stub fails the version handshake, not a workflow run. Have the stub write a sentinel, or assert its marker on stderr, and add a positive control where a succeeding stub gets past the probe | rust/crates/quoin-cli/tests/nfr_populations/commands.rs:110-147 |
| FND-005 | low | A plan that declares both `population_set` and `population_exempt` loads silently. The exemption is then meaningless and contradicts the set. Refuse the pair as `QM-PLAN-INVALID`, or state in FR-131-AC-7 which one wins | rust/crates/quoin-measurement/src/plans/design.rs:249-257 |
| FND-006 | low | Only `external_tool_failures` and its test carry `cfg(unix)`. On Windows, `workflow_commands`, `skill`, `FAILURE_MODES` and `WORKFLOW_ABOUT` are dead code and raise `dead_code` warnings. CI's Windows job builds but does not test or lint, so nothing is red today. Gate the whole NFR-007 section | rust/crates/quoin-cli/tests/nfr_populations/commands.rs:76-108 |
