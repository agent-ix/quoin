---
id: SR-20261117175261
title: "Code review of Quoin PLAT-1790 actor admission"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/src/analysis_evidence.rs, rust/crates/quoin-cli/src/assurance.rs, rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-cli/src/formal_rules.rs, rust/crates/quoin-cli/src/formal_rules_coverage.rs, rust/crates/quoin-cli/src/matrix.rs, rust/crates/quoin-cli/src/release/snapshot.rs, rust/crates/quoin-cli/src/run_observation.rs, rust/crates/quoin-quire/tests/analysis_counting.rs; PR #1004"
review_set: subset
---

## Summary

Reviewed PR #1004 at `98c9b2559a556211d919c6db65d5dfab6681f55b` against `origin/main`, including Rust behavior, refusal tests, CLI/verifier inputs, and the applicable PLAT-1790 criteria. The exact-pair and actor-mapping design is consistent with the examined criteria, but the new real-verifier tests are not runnable in the repository's test workflow because that workflow does not install the required EA executable/version.

## Verdict

**FAIL** — `make rust-test` unconditionally reaches a new test that requires a compatible external EA CLI, while `.github/workflows/build-test.yml` installs no such executable; the tested 0.8.1 CLI also lacks the requested `signature-verify` subcommand.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Real-verifier tests require an EA executable with `signature-verify`, but Quoin's test workflow does not install it; the available 0.8.1 CLI fails this test before it can verify the unsigned commit. Keep ordinary tests independent of the absent binary and place real EA integration in the authorized manual workflow, or use a documented supported executable distribution. Do not add an EA install to `build-test.yml` without a separate owner decision. | rust/crates/quoin-cli/src/analysis_evidence.rs:667-668; .github/workflows/build-test.yml:1-65 |

## Coverage

- Scope: all nine changed Rust files and the changed actor-admission paths; repository guidance, CI workflow, and FR-116/FR-153 acceptance text.
- Clean aspects examined: exact pair-introduction commit selection; mapped signer principal equality; exact commit verification result comparison; self-asserted success refusal; malformed, missing, tag, and unresolved ID refusal; CLI/config flow; test assertions and trace tags.
- Gate evidence: sanitized-path focused test `analysis_evidence::tests::actual_verifier_refuses_an_unsigned_pair_introduction` failed because `/home/peter/.cargo/bin/engineering-assurance` is v0.8.1 and reports `unrecognized subcommand 'signature-verify'`; the ordinary workflow contains no EA install step. `build-test.yml` is manual-dispatch only, and this review did not dispatch it.
- No edits or merges performed.
