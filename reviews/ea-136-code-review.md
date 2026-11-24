---
id: SR-20261124022828
title: "Code review — Quoin EA-136 expired-key approval test (PR #1043)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/tests/tc_plat_1789_signed_approval_chain.rs"
review_set: subset
---

## Summary

Ticket: EA-136. Reviewed Quoin PR #1043 (`git diff origin/main...HEAD`, one file, 6 insertions and 5 deletions). The PR changes the ignored end-to-end test at `tc_plat_1789_signed_approval_chain.rs:565`. The old test signed an approval tag with a tagger date backdated to 2019 against an allowed-signers entry with `valid-before="20200101"`, and asserted `untrusted_signer`. The new test tags with the current date, is renamed `an_expired_key_does_not_approve`, and keeps the healthy control unchanged.

## Verdict

**PASS (mergeable)** — the change is correct and the test now proves what its name says on every git version. Three low findings, none blocking.

Reasoning and evidence:

- Engineering Assurance FR-041 states that git 2.35 and later passes the claimed tagger date to `ssh-keygen` as `-Overify-time`, so `valid-before` is judged at the claimed time, while earlier git judges it at verification time. FR-037-AC-9 requires `signed_untrusted` for an expired entry only "on git 2.34.1". The old assertion ("whatever date the tagger claims") was therefore a property EA does not claim and git 2.35+ does not provide.
- Direct reproduction with OpenSSH 8.9p1 `ssh-keygen -Y verify` against an entry `valid-before="20200101"`: `-Overify-time=20190601000000` (the old backdated case on git 2.35+) gives `Good "git" signature`, exit 0; `-Overify-time=<now>` (the new case on git 2.35+) and no verify-time (git 2.34) both give `key has expired`, exit 255. The new tag date is the run time, so the key is refused on both code paths.
- The assertion `result["status"] == "untrusted_signer"` fails if an expired key were approved (status `approved`). The healthy control (same tag, same key, entry without expiry, expects `approved`) shows the only varied input is the expiry, so the refusal cannot be caused by a broken fixture.
- Ran against the real engineering-assurance 0.10.1 binary, git 2.34.1 and ssh-keygen 8.9p1: `cargo test --locked -p quoin-cli --test tc_plat_1789_signed_approval_chain -- --ignored`: `an_expired_key_does_not_approve ... ok`. A git 2.35+ binary is not installed on this machine, so the 2.35+ path was proven through `ssh-keygen -Overify-time` directly, not through git.
- No other assertion in the file depends on git 2.34: after the PR no test sets `GIT_COMMITTER_DATE` or any tagger date, and the only `valid-before` entry is this test's.
- Trace tags: no `#[trace(...)]` attribute changed; the test keeps `FR-188-AC-25`. Nothing in `spec/` or elsewhere references the old or new function name, so the rename has no spec or matrix impact.
- Rust lane: `rustfmt --check --edition 2024` on the file passes; focused clippy on the test target via locked-build.sh reports no warning in the touched file. With `-D warnings` the run stops earlier, on 13 pre-existing lints in the untouched dependency crate `quoin-evidence` (`timestamp.rs`, `store/records.rs`, `store/gc.rs`, `qualify/runs.rs`: indexing_slicing, missing `# Errors`, match_same_arms, must_use, too_many_lines). Without `-D`, the test target compiles, and every warning it prints comes from `quoin-evidence`.
- Out of PR scope, observed in the same run: the 7 `formal-rules status` tests in this file fail against engineering-assurance 0.10.1 with `signature_verify_request_invalid: signature-verify request is not an exact signature-verify request`. These tests are untouched by the PR; it is a quoin/EA wire mismatch to raise separately.

## Review Notes

- quoin 0.36.8; spec-artifacts-process v0.36.0.
- Methods: code-review with the Rust lane. gap-analysis skipped: the diff changes no production code (one test file only). No spec or plan file changed, so no spec-review.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low      | `approval_tag`'s `env` parameter is now dead: every call site passes `&[]`; the expired-key test was its only non-empty user | rust/crates/quoin-cli/tests/tc_plat_1789_signed_approval_chain.rs:365 |
| FND-002 | low      | The new healthy-control comment is one 120-character line, unlike the wrapped comments around it (rustfmt does not wrap comments, so the check passes) | rust/crates/quoin-cli/tests/tc_plat_1789_signed_approval_chain.rs:594 |
| FND-003 | low      | Pre-existing, not introduced here: the expired-key test is tagged `FR-188-AC-25`, but that AC covers the `formal-rules status` signature-verify path, not `approval-verify` key expiry, so the tag credits AC-25 with a test that does not exercise it | rust/crates/quoin-cli/tests/tc_plat_1789_signed_approval_chain.rs:562 |
