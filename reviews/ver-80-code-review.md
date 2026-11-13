---
id: SR-20261113055214
title: "Code review (Rust lane) — VER-80 differential-report v2 adapter (quoin PR 850)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/Cargo.toml, rust/Cargo.lock, rust/deny.toml (context), rust/crates/quoin-evidence/Cargo.toml, rust/crates/quoin-evidence/src/adapters/differential_report.rs, rust/crates/quoin-evidence/src/adapters/registry.rs, rust/crates/quoin-evidence/tests/tc_differential_report_v2.rs, rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs, rust/crates/quoin-cli/src/evidence.rs (context), rust/crates/quoin-core/src/ops/evidence/wire.rs (context), tests/fixtures/evidence/README.md, tests/fixtures/evidence/differential-report-v2/*.json"
review_set: subset
---

## Summary

Ticket: VER-80. PR head `e6c29c90`, diff `git diff origin/main...HEAD`. Rust lane (rust-style)
folded in. Focused checks run on this head: `cargo test -p quoin-evidence --test
tc_differential_report_v2` (10 passed), `cargo test -p quoin-core --test
tc_ver80_differential_report_record` (4 passed), `cargo clippy -p quoin-evidence --all-targets
--all-features -D warnings` (clean), `cargo fmt --check` (clean), `cargo check -p quoin-evidence
--target x86_64-pc-windows-msvc` with stub CC/AR (passes), `cargo deny check` (fails), and the
built `quoin evidence record` binary over the fixtures.

The adapter itself is small and correct: it bounds input by `quire_analyze::MAX_REPORT_BYTES`
before any parse, delegates acceptance to `validate_report_document`, maps the five dispositions
as FR-151's table states, names `disagreement` as an unrepresented result, carries no trace ids,
copies no schema, and its defensive errors cannot panic. The v1 reader and its tl-mltl, r2u2 and
c2po tool claims are fully removed; the only remaining mentions are the inventory doc's correct
"Not covered" row for tl-mltl's live outputs and an unrelated comment in tc_1322.

## Verdict

Not mergeable at this head. FND-001 fails `make rust-gate` and the CI `rust-deny` job. FND-002 is
a spec claim (FR-151-AC-3, FR-069-AC-6) the shipped command violates, reproduced. FND-003 is a
real edge on the size bound. The rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `cargo deny check` fails on the new quire-analyze subtree (licenses, sources, bans), and `rust-deny` is in `make rust-gate` and in CI. Legitimate fixes in the file's own pattern: add allow-git entries for quire-analyze, quire-contract-ir, quire-verification-contracts, quire-exact, quire-walk and quire-semantic-value; add named exceptions for quire-verification-contracts (AGPL-3.0-or-later) and ar_archive_writer (Apache-2.0 WITH LLVM-exception). With only these, licenses and sources pass (measured with a candidate config). The bans failures need more. ix-trace-rs and quire-canonical are duplicated because quoin pins `rev` while upstream uses `branch = "main"`. Both pins fast-forward to main (6 and 11 commits), so converge by switching quoin's two workspace entries to `branch = "main"`. Skipping them would exempt our own root pins, which the file forbids. The jsonschema 0.17.1 and 0.55.1 subtrees and typify's schemars 0.8 subtree would be a fourth concession group, and the file says such a group is escalated, not skipped. Fix upstream, or get a ruled exact-version skip group with a removal condition. | rust/deny.toml; rust/Cargo.toml:80-82; rust/Cargo.lock |
| FND-002 | high | `quoin evidence record` human output does not print the unrepresented result. On disagreement.json the command prints only `recorded S1 → <run path>` and exits 0. JSON output carries `unrepresented`. FR-151-AC-3 and FR-069-AC-6 require it in both, and docs/campaign-native-result-inventory.md shows a `not transcribed — …` line no code emits. The `Some("run")` arm of the renderer ignores `payload.unrepresented`. | rust/crates/quoin-cli/src/evidence.rs:167-173 |
| FND-003 | medium | The report passes through the `evidence.record` request bound, `MAX_RECORD_BYTES` = 48 MiB (the serialized request, obligations included), before it reaches the adapter's 64 MiB `MAX_REPORT_BYTES`. A report the producer validly publishes at about 48 to 64 MiB is refused by the command. TC-001 step 3 (an exactly-MAX input is "not refused for size", run through the command) cannot pass. The CLI also reads `--results` whole, with no bound, before any check. Fix: make the record bound cover `quire_analyze::MAX_REPORT_BYTES` plus an obligation allowance, and read `--results` with `take(limit + 1)`; or state in FR-151/NFR-032 that the command's effective bound is the request bound. | rust/crates/quoin-core/src/ops/evidence/wire.rs:99; rust/crates/quoin-cli/src/evidence.rs:351-353 |
| FND-004 | low | The dependency is heavy for one function and one constant: 40 new lock entries, including psm/stacker (native assembly build), typify build-time codegen, and five transitive agent-ix git crates on `branch = "main"`. quire-contract-model's exact `unicode-ident = "=1.0.24"` forces a workspace-wide downgrade from 1.0.26 (harmless table data, but quoin's graph is now set by an upstream exact pin). The Windows evidence (`cargo check` with stub CC/AR, reproduced) proves the Rust code type-checks for msvc, not that psm's assembly builds and links; only the native-release windows-msvc job proves that. A lean validator crate or a default-off runner feature upstream would remove all of this and most of FND-001. | rust/Cargo.lock; rust/crates/quoin-evidence/Cargo.toml:31 |
| FND-005 | low | rust/deny.toml keeps a `quire-rs` licence exception that never matches: quire-rs is `publish = false`, so `private.ignore` already covers it, and cargo-deny warns `license-exception-not-encountered`. This is a dead entry and can be deleted in the same deny.toml edit. It predates this PR. | rust/deny.toml:60-62 |
| FND-006 | low | The determinism test changes the process-wide working directory with `set_current_dir` in a test binary that runs tests in parallel. If the assertion panics, the directory is never restored. The test also varies neither the environment nor the path, both of which AC-11 names. The adapter takes bytes, so the check is weak but harmless. | rust/crates/quoin-evidence/tests/tc_differential_report_v2.rs:398-408 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| `parse_differential_report` size check before parse | examined | Correct; tested at MAX and MAX+1 |
| `refusal` naming the found `schema` value | examined | Correct for v1, v3, retired, absent, non-string |
| disposition match and `describe_engine` | examined | Matches the FR-151 table; unreachable arms return errors, no panic |
| registry entry (tools `differential-report`, `quire-analyze`) | examined | Correct; retired tools fall back to `entries` |
| v1 reader removal across rust/, tests/, docs/, skills/ | examined | Nothing dangling |
| fixtures and README | examined | Byte-exact captures with `/tmp/quire-analyze-run-package-*` executables and `/stub` identity; acceptable |
| `rust/Cargo.toml` quire-analyze `rev = 27c4fbb1` | examined | Equals quire-analyze `main` at review time |
| cargo deny, Cargo.lock | examined | FND-001, FND-004 |
| quoin-cli `record_run` rendering | examined | FND-002 |
| `MAX_RECORD_BYTES` | examined | FND-003 |

Rust lane (rust-style): no `unwrap` or `expect` in production code, errors go through the existing
`EvidenceError::adapter`, `#[implements("FR-151")]` is present, there is no unsafe code, and there
is no new public API beyond the existing function.
