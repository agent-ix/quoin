---
id: SR-20261113082615
title: "Code review (Rust lane) — VER-106 differential-report admitted to evidence record --discharges (quoin PR 856)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-core/src/ops/evidence/records.rs, rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs, rust/crates/quoin-cli/src/help.rs (context), rust/crates/quoin-cli/src/evidence.rs (context), rust/crates/quoin-cli/tests/fixtures/retained-command-help.json (context), rust/crates/quoin-cli/tests/tc_retained_help_capture.rs (context)"
review_set: subset
---

## Summary

Ticket: VER-106. PR head `7c47d6ea`, diff `git diff origin/main...HEAD` (2 files). The Rust lane
(rust-style) is folded in. Focused checks run on this head:
`cargo test -p quoin-core --test tc_ver80_differential_report_record` (11 passed),
`cargo clippy -p quoin-core --all-targets --all-features -- -D warnings` (clean),
`cargo fmt -p quoin-core --check` (clean). The built `quoin` binary was run over the
differential-report fixtures in a temporary repository.

The production change is one guard widened from `"sbom"` to `"sbom" | "differential-report"`, plus
the refusal message and the comment. It is correct. Reproduced: junit and entries with
`--discharges` are refused with exit 1, the message names the adapter, and no run is written. A
satisfied report with `--discharges FR-001-AC-2, FR-001-AC-1,NOPE-1` binds both ids and reports
`NOPE-1` as unmatched. A disagreement prints the `not transcribed` line in human output.

## Verdict

The code is correct. One user-facing defect remains: FND-001, the shipped `--help` text. The other
findings are about test strength. There is no unsafe code, no new public API and no panic path, and
the error goes through the existing `CoreError::new(BadRequest)`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `quoin evidence record --help` page still says `--discharges` "Applies to finding-shaped recordings and the sbom adapter, and is refused on every other run adapter". That is now false for differential-report. This is the real help text, not a test-only copy: `help.rs:136` compiles `tests/fixtures/retained-command-help.json` into the binary with `include_str!`, and `retained_help` serves that page for `evidence record --help`. Reproduced with the built binary. The clap `Arg::new("discharges")` has no help attribute. `tc_retained_help_capture` compares the binary's output with the same file, so it cannot catch this. Fix: edit that one paragraph in the fixture's `evidence record --help` stdout to name the differential-report adapter. No docs/ or skills/ file states the adapter set. | rust/crates/quoin-cli/tests/fixtures/retained-command-help.json:311; rust/crates/quoin-cli/src/help.rs:136 |
| FND-002 | medium | `tc_002_a_stated_obligation_binds_a_satisfied_report` takes the expected binding symbol from the stored entry (`symbol_of(&run)`). So FR-152-AC-1's "symbols are exactly `<analysisKind>::<requestDigest>`" cannot fail: a wrong symbol on the entry would be bound and the test would still pass. Fix: build the expected symbol from the fixture's `analysisKind` and `requestDigest` fields and compare both the entry and the binding with it. | rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs:194-211 |
| FND-003 | low | The comment in `tc_002_no_stated_obligation_leaves_the_entry_unbound` says the obligation ids are "unrelated to the symbol, analysisKind and requestDigest". But `consistency` equals the fixture's `analysisKind`. The test is stronger than the comment, because an id equal to the kind still binds nothing, but the comment is false and FR-152-AC-3 asks for ids that differ. Fix: correct the comment, or split it into a differing-ids case (AC-3) and an equal-to-kind case (NFR-033). | rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs:219 |
| FND-004 | low | The admission set is two name literals in quoin-core (`matches!(adapter.name, "sbom" \| "differential-report")`), but the adapter registry in quoin-evidence owns the names. If an adapter is renamed, it silently falls into the refused set. A `states_obligations: bool` field on `RunAdapter`, set in the registry, would keep the rule next to the adapter. This is optional. | rust/crates/quoin-core/src/ops/evidence/records.rs:219 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| `record_entries` guard and refusal message | examined | Correct; junit and entries refused naming the adapter, no run written (reproduced) |
| `stated_obligations` trim/sort/dedup (unchanged) | context_only | Read to check AC-6 behaviour |
| quoin-cli `record_run` `--discharges` parsing (single value, comma split) | context_only | Read for the help text and AC-6 |
| help source (`help.rs` RETAINED_HELP, fixture) | examined | FND-001 |
| new tests in tc_ver80_differential_report_record.rs | examined | They assert store files and payloads, not internals; FND-002 and FND-003 |

Rust lane: no `unwrap` or `expect` outside tests, `#![allow]` carries a reason, nothing new is
public, and clippy and fmt are clean.
