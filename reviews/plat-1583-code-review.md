---
id: SR-20261113150300
title: "Code review (Rust lane) — PLAT-1583 FR-175 anomaly check fix-verification evidence (quoin PR 874)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-anomaly/src/verification.rs, rust/crates/quoin-anomaly/src/check.rs, rust/crates/quoin-anomaly/src/record.rs, rust/crates/quoin-anomaly/src/refs.rs, rust/crates/quoin-anomaly/src/lib.rs, rust/crates/quoin-anomaly/Cargo.toml, rust/Cargo.lock, rust/crates/quoin-cli/src/anomaly.rs, rust/crates/quoin-cli/tests/tc_175_fix_verification_command.rs, rust/crates/quoin-cli/tests/tc_162_anomaly_report_command.rs (context), rust/crates/quoin-evidence/src/store (context)"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-175", type: references}
  - {target: "ix://agent-ix/quoin/FR-136", type: references}
  - {target: "ix://agent-ix/quoin/FR-030", type: references}
---

## Summary

Ticket: PLAT-1583. PR head `ed393a84`, diff `git diff origin/main...HEAD` (9 files). The Rust lane
(quoin's `.claude/skills/rust-style`) is folded in. Checks run on this head, in a scratch detached
worktree: `cargo test -p quoin-anomaly` (49 passed), `cargo test -p quoin-cli --test
tc_175_fix_verification_command --test tc_136_anomaly_command --test tc_162_anomaly_report_command`
(15 + 5 + 9 passed), `cargo clippy -p quoin-anomaly -p quoin-cli --all-targets --all-features -D
warnings` (clean), `cargo fmt --check` (clean). The built `quoin` was run by hand over a temporary
scope built from the spec-artifacts-process fixtures.

Store reads go through quoin-evidence's own functions (`list_recorded_suites`, `read_runs`,
`DiskEvidence`), nothing re-parses run JSON. The new `quoin-anomaly -> quoin-evidence` edge has no
cycle (quoin-evidence depends on quoin-quire-types, quoin-store, quoin-yaml only) and Cargo.lock
carries it (`--locked` builds). The `Severity::Warning` and `Diagnostic.target` additions change no
JSON or snapshot: `anomaly check` emits rendered text only, and no consumer serialises the anomaly
`Diagnostic`. Criterion matching uses quire's obligation `target_ids` via `quire.coverage`, never a
Verification cell, and drops a target id that is itself an obligation id as FR-030-AC-15 does.

## Verdict

Request changes. The FR-175 derivation is correct and well tested, but two behaviours sit outside
the spec: FND-001 (an invented degraded mode that hides a failing criterion behind a stderr note)
and FND-002 (unreadable run files dropped without the report FR-030-AC-13 requires). FND-003 and
FND-004 are small cleanups. No unwrap or panic in production code, no unsafe, no digests, pins or
SHAs, no compatibility layer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `run_check` turns a `quire.coverage` failure into a stderr diagnostic `anomaly.obligations_unavailable` and carries on with Criterion rows matched on their own id only. FR-175 defines no such code and no degraded mode, the code is in no catalogue, and no test exercises it. Its only reason is tc_162's FR-136-AC-13 test passing `--module .`, which is not a module root (quire: "manifest.yaml not found in module root"). Reproduced: a `Criterion FR-001-AC-1` row with a failing `TC-001` run gives `warning anomaly.fix-result-failing` with valid modules or with no `--module`, and gives no finding (only the stderr note) with `--module .`. So an unloadable module root silently suppresses the finding FR-175-AC-8 requires. Value test: it is not a feature, it is a fallback added to keep a wrong test argument working. Spec-conformant behaviour: propagate the coverage failure as `quoin evidence record` does (the check fails, it does not answer partially), and fix tc_162 to pass a real module root (the spec_artifacts_process module plus the hazard manifest, as tc_175 does) or no `--module`. If a degraded mode is wanted, it needs an FR-175 CR first. | rust/crates/quoin-cli/src/anomaly.rs:143-157; rust/crates/quoin-cli/tests/tc_162_anomaly_report_command.rs:415 |
| FND-002 | medium | `read_results` collects the unreadable run files into `unreadable` and discards them, and `let Ok(runs) = read_runs(..) else { continue }` also swallows a StoreIo failure, which quoin-evidence deliberately keeps distinct from an unreadable file. FR-175 says "FR-030-AC-13 reports the unreadable file", and FR-030-AC-13 says one unreadable run file is "skipped and reported rather than fatal". Reproduced: a run file missing its `symbol` field leaves the check output with no word about it; a `Test` row then reads `fix-test-missing-result` with no stated cause. Fix: report each skipped path (and an I/O failure) as a diagnostic naming the file and cause, without changing the exit status; AC-6's test should assert it. | rust/crates/quoin-anomaly/src/verification.rs:178-199 |
| FND-003 | low | `Diagnostic.target` is written (`finding.target = Some(..)`) and never read: the rendered line takes the target from the message text. A public field on a `Serialize` struct that nothing consumes is dead API. Render the target from the field (and keep the message free of it), or drop the field. | rust/crates/quoin-anomaly/src/check.rs:113-114; rust/crates/quoin-anomaly/src/verification.rs:150 |
| FND-004 | low | `read_runs(..)` followed by `.into_iter().next_back()` re-implements quoin-evidence's `latest_run`, which is exactly `read_all(..).pop()` with the same `skipped` out-parameter. Call `latest_run(&store, &suite, &mut unreadable)` so "newest by timestamp" has one owner. | rust/crates/quoin-anomaly/src/verification.rs:185-193 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| `Verification::result_of` (failing wins, pass, skip-only missing) | examined | Correct; mutants M1, M10, M11 killed |
| `read_results` newest run per suite, unreadable suite contributes nothing | examined | Derivation correct (M2, M4 killed); FND-002, FND-004 |
| `Verification::finding` row evaluation (dangling skip, Test any corpus id, Criterion AC only) | examined | Conforms to FR-175 statements; M7, M8 killed |
| `is_fixed` / `evaluates_criteria` | examined | Correct; M3 killed |
| `DiagnosticCode` new codes, `severity()`, `Severity::Warning` | examined | Codes, reasons, severities match the FR-175 table; M9 killed |
| `Diagnostic.target` | examined | FND-003 |
| `record.rs` `## Fix Verification` reader | examined | Columns, `-`/empty target, mapping errors via `table()` as the CR note says |
| `refs.rs` `test_case_ids`, `CorpusIds` | examined | Correct; widens `refs` resolution as the CR note says |
| `quoin-cli` `run_check`, `criterion_tests` | examined | FND-001; obligation-id filtering matches FR-030-AC-15 |
| Cargo.toml / Cargo.lock dependency edge | examined | No cycle, lock consistent |
