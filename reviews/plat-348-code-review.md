---
id: SR-20261111191351
title: "Code review of quoin PR 809 (cargo-deny adapter, SBOM --discharges)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@df41e4a3; rust/crates/quoin-evidence/src/adapters/cargo_deny.rs, rust/crates/quoin-evidence/src/adapters/registry.rs, rust/crates/quoin-evidence/src/adapters/mod.rs, rust/crates/quoin-core/src/ops/evidence/records.rs, rust/crates/quoin-core/src/ops/evidence/wire.rs, rust/crates/quoin-evidence/tests/tc_cargo_deny.rs, rust/crates/quoin-evidence/tests/fixtures/cargo-deny-*.jsonl, rust/crates/quoin-core/tests/tc_plat348_finding_and_sbom_record.rs, spec/matrix.md, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json (unchanged, served help)"
review_set: subset
---

# Code review of quoin PR 809

## Summary

Tickets: PLAT-348, PLAT-347. Scoped to `git diff origin/main...HEAD` at df41e4a3. The Rust
lane (`rust-review`, with the repo `rust-style`) is folded in. Evidence gathered through
`locked-build.sh` in a fresh worktree: the five touched test targets pass, and
`cargo clippy -p quoin-evidence -p quoin-core --all-targets --all-features -D warnings` exits 0.
Real cargo-deny 0.19.8 runs over a scratch crate, and a throwaway probe test (not
committed) through `evidence.record`, confirmed FND-001, FND-003 and FND-004.

## Verdict

**NOT MERGEABLE AS IS (one high, four medium, two low).** What is right: a run with no
summary is refused, and a real failed run (bad config, bad manifest, missing advisory db)
prints no summary, so those cases are refused. The input is bounded by `MAX_RECORD_BYTES`
(48 MiB) at the boundary. Non-UTF-8 is refused by the CLI. serde_json's recursion limit
refuses deep nesting. So the findings vector has a ceiling through the request bound,
as cargo-audit and sarif do. `rules_evaluated` stays `None`, so vacuity stays silent.
The fixtures look like real tool output: the `helps:2` with no help lines matches
default `--log-level warn`. They carry no machine paths or secrets. `cargo-audit` no
longer claims `cargo-deny`. The module-size cap holds (records.rs 495, registry.rs 408,
under 700). IDs TC-2700..2705, FR-034-AC-21..24 and FR-041-AC-7/8 do not collide with
origin/main or open PRs #806/#807/#808 or the PLAT-308 worktree. `quoin matrix` reports no
`criterion-tagged-outside-row`.

The defect is that the summary is taken as proof without reading it. A summary is also
printed by a run that found errors and printed none of them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The summary's counts are never read, so a FAILED run reads as a clean scan and discharges. Real `cargo deny --log-level off --format json check licenses` (exit 4) prints only `{"fields":{"licenses":{"errors":1,...}},"type":"summary"}`. Through `evidence.record` with `--discharges SCA-1` this returns `findings: [], bound: ["SCA-1"], vacuous: false`. Diagnostics skipped by `diagnostic_finding` (no advisory id and no code) vanish the same way. Refuse when the summary's errors+warnings across checks exceed the error/warning diagnostics read, and test it with that real line | rust/crates/quoin-evidence/src/adapters/cargo_deny.rs:253-284 |
| FND-002 | medium | The summary's check set is ignored. `{"type":"summary","fields":{}}` (no check ran) is accepted as proof, and TC-2703 records exactly that. A real `cargo deny check bans` summary names only `bans`, yet it records as a clean cargo-deny scan and binds an advisory obligation stated with `--discharges`. Refuse a summary that names no check, and record the checks it names (for example in `ruleset`) so the scan says what it covered | rust/crates/quoin-evidence/src/adapters/cargo_deny.rs:264,279-284 |
| FND-003 | medium | `--discharges` is now applied on EVERY run adapter (junit, entries, agent-eval, cargo-mutants...), not only sbom, which is the only adapter FR-041 specifies. Probe: a JUnit run with one passing and one failing test plus `--discharges FR-9` returns `bound: ["FR-9"]`, binding the stated obligation to every passing test of a red suite. Previously the flag was ignored on this path. No FR states this. Restrict injection to the sbom adapter and refuse `--discharges` on other run adapters with a named error, or specify and test the general behaviour | rust/crates/quoin-core/src/ops/evidence/records.rs:210-226 |
| FND-004 | medium | FR-041-AC-8 (an unknown `--discharges` id is reported `unmatched`) fails for an empty inventory. The ids are injected per entry, so with zero components none reaches `record_run`'s unmatched set. Probe: `components: []` with `discharges: ["NO-SUCH-1"]` returns `unmatched: []`. Check stated ids against `request.obligations` directly, as `record_scan` does for `unknown`, and test the empty case | rust/crates/quoin-core/src/ops/evidence/records.rs:215-226 |
| FND-005 | medium | `quoin evidence record --help` is stale. help.rs serves `retained-command-help.json` verbatim, and TC-1650 only replays that file, so no test can catch drift. The page omits `cargo-deny` from the USAGE `--adapter` list, the FLAGS prose list and `<options: ...>`, and still says `--discharges` is "Finding-shaped only". Edit the `["evidence","record","--help"]` case stdout: append `cargo-deny` to all three adapter lists, and reword `--discharges` to cover the SBOM path (as FND-003 settles it). Prior PRs (#757, #763, #766) hand-edit this fixture the same way | rust/crates/quoin-cli/tests/fixtures/retained-command-help.json |
| FND-006 | low | The TC-2704 matrix row cites `tc_2704_an_sbom_with_no_stated_obligation_binds_nothing`, but the function is `tc_2704_an_sbom_with_no_stated_with_no_stated_obligation_binds_nothing` (words duplicated). Rename the function | rust/crates/quoin-core/tests/tc_plat348_finding_and_sbom_record.rs:203 |
| FND-007 | low | The summary need not be unique or last: diagnostics after it, and a second summary, are accepted. So output from an unfinished run concatenated with a finished run's reads as one finished scan. Require exactly one summary, on the last non-empty line | rust/crates/quoin-evidence/src/adapters/cargo_deny.rs:253-272 |
