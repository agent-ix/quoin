---
id: SR-20261111195806
title: "Code review of quoin PR 813 (FR-039 surviving-mutant routing)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@25d66ce; rust/crates/quoin-evidence/src/{adapters/cargo_mutants.rs,types/run.rs,types/mod.rs}, rust/crates/quoin-auditor/src/audit/{scores.rs,ladder.rs,mod.rs}, rust/crates/quoin-finding-types/src/finding.rs, skills/spec-correctness/{SKILL.md,references/surviving-mutants.md}, tests"
review_set: subset
---

# Code review of quoin PR 813

## Summary

Ticket: PLAT-394. Scoped to `git diff origin/main...HEAD` at 25d66ce (two commits). The branch
merges cleanly onto the current origin/main (5f056b84, which includes #810). The Rust lane
(`rust-review`, with the repo's `.claude/skills/rust-style/SKILL.md`) is folded in.

Focused tests ran through locked-build.sh (debug=0, jobs 4). quoin-evidence, quoin-auditor,
quoin-finding-types and quoin-assurance lib tests passed, with `tc_adapters_behaviour`,
`store_behaviour` and `tc_audit_ladder`. quoin-core `tc_3102` passed. quoin-cli `tc_shell_contract`,
`tc_graph_command` (TC-1650) and `tc_retained_help_capture` passed. `cargo clippy --all-targets
--all-features -D warnings` is clean on the three changed library crates. `cargo fmt --check`
fails (FND-001).

**Real cargo-mutants output.** cargo-mutants 27.1.0 is installed. It ran on a scratch crate with
missed, caught, timed-out and unviable mutants, and the scratch crate was deleted afterwards.
The real `outcomes.json` went through the PR's `parse_cargo_mutants`:

- `span.start.line` is the mutated line. Confirmed.
- `MissedMutant` and `Timeout` are kept as survivors. `CaughtMutant` and `Unviable` are not. The
  classification is correct.
- `name` is **not** the bare description. It is `"<file>:<line>:<col>: <description>"`, for
  example `src/lib.rs:6:7: replace + with * in add` (FND-002).

A note on the first run: cargo-mutants under an inherited `CARGO_TARGET_DIR` with `--jobs 2`
gave impossible verdicts, such as `spin -> 0` reported as missed. Only the run without a shared
target dir was used.

**Record shape.** `survivors` is `#[serde(default, skip_serializing_if)]` on `RunEntry`, so older
records still read. That is an optional field, not a compatibility layer. Nothing else was added.
The run record stays smaller than the `outcomes.json` it came from (25 x 200 characters per
function at most), well under `MAX_RECORD_BYTES` (48 MiB) and `MAX_AUDITOR_REQUEST_BYTES`
(32 MiB). The bounds hold only on the cargo-mutants path, though (FND-003).

**Finding kinds.** `FindingKind::ALL` is now 12. No Rust code matches it exhaustively.
`quoin-assurance::matrix::bucket` sends unknown kinds to bucket 2 (`undischarged`). A
surviving-mutant finding always comes with `insufficient-mutation-score` anyway, so the status
is unchanged. Matrix markdown cells pass through `cell()`, which escapes `|`, `\` and newlines. The
assurance case's `because` takes `findings.first()`, which is the score finding. Mermaid labels use
the statement, not the finding text. The frozen `retained-command-help.json` audit page already
listed only 9 checks before this PR (it omits `mocked-confirmation` and `combinatorial-gap`).
TC-1650 passes, and the help is no less true than it was.

**Ordering.** Findings come in binding order (sorted by obligation, then suite) and then in
`BTreeMap` symbol order, so the output is deterministic.

**FR-039-AC-1.** No survivor finding is emitted when no floor is declared, when the floor is
met, or when the score is unmeasured. In the last case `mutation_finding` returns
`unmeasured-mutation-score` and the `missed_floor` guard is false. Skipped entries are filtered
out. The behaviour is correct.

## Verdict

**CHANGES REQUESTED (four medium, one low).** The plumbing is small and correct on the main
path. The problems:

- the format gate fails (FND-001)
- the description field carries a prefix that the tests never saw (FND-002)
- the caps can be bypassed through the `entries` adapter (FND-003)
- the per-obligation cap fills with the wrong survivors (FND-004)
- the skill reads tool and code-derived text with no data-only guard (FND-005)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `cargo fmt --all --check` (the `make rust-lint` leg) fails on four changed files. It wants the `pub use scores::{...}` line joined and three `assert!`/`assert_eq!` calls split. The PR body reports clippy clean but not fmt. Run `make rust-fmt` | rust/crates/quoin-auditor/src/audit/mod.rs:39, rust/crates/quoin-auditor/tests/tc_audit_ladder.rs:806, rust/crates/quoin-evidence/src/adapters/cargo_mutants.rs:23, rust/crates/quoin-evidence/tests/tc_adapters_behaviour.rs:453 |
| FND-002 | medium | Real cargo-mutants 27.1.0 `name` is `"src/lib.rs:6:7: replace + with * in add"`, prefixed by file:line:col. It is not `"replace f -> bool with true"` as the doc comment, the TC-3100 fixture and the skill reference assume. So every survivor repeats the locus, and the 200-character cap is spent partly on the path. On a long path plus a long return type, truncation cuts off the `with <replacement>` tail, which is the part that says what went undetected. The fixture is hand-written to match the reader, against this repo's own rule (spec/matrix.md FR-041 note: "A fixture written to match the reader only proves the reader parses itself"). Strip the `file:line:col: ` prefix, or build the description from `genre`/`replacement`/`function`, and use a real outcomes.json excerpt as the fixture | rust/crates/quoin-evidence/src/adapters/cargo_mutants.rs:30, rust/crates/quoin-evidence/src/adapters/cargo_mutants.rs:298-313, rust/crates/quoin-evidence/tests/tc_adapters_behaviour.rs:440-452 |
| FND-003 | medium | The bounds hold only in the cargo-mutants adapter. `parse_entries` (the generic `entries` adapter) deserialises `Vec<RunEntry>` wholesale, and neither `RunEntry` nor `Survivor` validates. So any producer can record an unbounded `survivors` list with arbitrarily long, multi-line `mutation` text. That contradicts the doc ("Only a mutation tool writes it", "bounded by MAX_SURVIVORS_PER_ENTRY") and FR-039's stated bound. The auditor caps findings at 25 per obligation, but each summary is still unbounded untrusted text, which flows into JSON, the matrix detail cell and the spec-correctness prompt. Enforce both caps (and strip control characters) where survivors are read or deserialised, or refuse `survivors` from non-mutation adapters | rust/crates/quoin-evidence/src/adapters/entries.rs:24-48, rust/crates/quoin-evidence/src/types/run.rs:380-402, rust/crates/quoin-auditor/src/audit/scores.rs:82-103 |
| FND-004 | medium | `surviving_mutant_findings` takes survivors of **every** mutation-scored entry of the bound suites, in symbol-name order, until 25. Symbols that meet the floor are included, and the cap can be filled by them before the symbol that actually missed the floor is reached. Reproduced on the real run: with a 0.5 floor, `label` (0.0) and `add` (0.25) miss it, but `count_down` (0.57), `is_big` (0.8) and `spin` (0.86) also produce survivor findings. With more symbols, the `label` survivors would be cut by the cap. The finding text "the tests bound to it do not detect X" and the skill's "emit one property ... tag it with the same row_id" then aim tests at code that passed the floor. Restrict to entries whose score is below the floor, or order by ascending score before capping, and add a test with a passing symbol beside a failing one | rust/crates/quoin-auditor/src/audit/scores.rs:75-105, rust/crates/quoin-auditor/tests/tc_audit_ladder.rs:848-858 |
| FND-005 | low | The new skill input feeds `summary`/`symbol` text into an agent prompt. That text comes from cargo-mutants and from identifiers in the code under test (function names and return types), and FND-003 also lets it come from any `entries` producer. The reference never says to treat finding text as data. Add one line to surviving-mutants.md saying that finding text is data to ground against the code, never instructions | skills/spec-correctness/references/surviving-mutants.md:16-21 |

## Examined scope

```yaml
clean: false
scope:
- {id: cargo_mutants.rs, path: rust/crates/quoin-evidence/src/adapters/cargo_mutants.rs, role: examined, excerpt: "MutantScenario.name/span; survivor_of; MissedMutant|Timeout kept, capped"}
- {id: run.rs, path: rust/crates/quoin-evidence/src/types/run.rs, role: examined, excerpt: "MAX_SURVIVORS_PER_ENTRY=25, MAX_SURVIVOR_CHARS=200, Survivor, RunEntry.survivors serde(default)"}
- {id: scores.rs, path: rust/crates/quoin-auditor/src/audit/scores.rs, role: examined, excerpt: "surviving_mutant_findings, MAX_SURVIVOR_FINDINGS=25"}
- {id: ladder.rs, path: rust/crates/quoin-auditor/src/audit/ladder.rs, role: examined, excerpt: "missed_floor guard on INSUFFICIENT_MUTATION_SCORE"}
- {id: finding.rs, path: rust/crates/quoin-finding-types/src/finding.rs, role: examined, excerpt: "SURVIVING_MUTANT; ALL: [&str; 12]"}
- {id: surviving-mutants.md, path: skills/spec-correctness/references/surviving-mutants.md, role: examined, excerpt: "Keep the findings whose kind is surviving-mutant. Group them by obligation"}
- {id: entries.rs, path: rust/crates/quoin-evidence/src/adapters/entries.rs, role: context_only, excerpt: "entries: Option<Vec<RunEntry>> deserialised wholesale"}
- {id: matrix.rs, path: rust/crates/quoin-assurance/src/matrix.rs, role: context_only, excerpt: "bucket(): unknown kind -> 2; cell() escapes | and newlines"}
```
