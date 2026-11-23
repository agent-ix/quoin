---
id: SR-17092
title: "Code review (with Rust lane) of PLAT-1839 module lag warning"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-modules/src/lag.rs, rust/crates/quoin-modules/src/error.rs, rust/crates/quoin-modules/src/lib.rs, rust/crates/quoin-modules/Cargo.toml, rust/crates/quoin-cli/src/module.rs, rust/crates/quoin-cli/src/catalog.rs, rust/crates/quoin-cli/src/help.rs, rust/crates/quoin-cli/tests/tc_1839_module_lag.rs, rust/crates/quoin-core/src/ops/modules/taxonomy.rs, rust/Cargo.lock; PR agent-ix/quoin#1027"
review_set: subset
---

## Summary

Ticket: PLAT-1839. Code review with the rust-review / rust-style lane folded in, scoped to `git diff origin/main...HEAD` (commits 0ae5e0be, 05bcd73c).

The change adds `quoin_modules::lagging_modules` (semver compare of the registry's recorded install `ref` against the manifest pin), a `QM032_MODULE_LAGS_PIN` code, a `quoin module doctor` subcommand and lag diagnostics appended to `quoin catalog validate`. `semver` is already a workspace dependency (`=1.0.28`, used by quoin-cli and quoin-delivery); no new third-party crate enters the lock.

Measured on terra (read-only `quoin module list`): spec-artifacts-iso is installed at `v0.25.0` while the branch pins `v0.27.0`; the iso commits between those tags include PLAT-1526/1577/1578 (software safety class). engineering-assurance is installed at `v0.8.2` against a `v0.10.1` pin. The comparison therefore does catch the ticket's motivating case on this machine.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `QM032_MODULE_LAGS_PIN` is not added to `rust/crates/quoin-modules/ERROR_CODES.md`; `tc_381_202_the_error_code_catalogue_round_trips_and_is_unique` asserts every `ModulesErrorCode::all()` entry is documented there, so that existing test fails on this branch. The coder's `--lib lag` filter does not run it. | rust/crates/quoin-modules/src/error.rs:123, rust/crates/quoin-modules/src/error.rs:593-600 |
| FND-006 | medium | `cargo clippy -p quoin-modules -p quoin-cli --all-targets` reports `clippy::indexing_slicing` ("indexing may panic") at the three `lags[0]` assertions; the repo gate (`Makefile:40`) runs clippy with `-D warnings`, so the PR fails it. The test module's `#[allow]` covers only `unwrap_used`; use `let [lag] = lags.as_slice() else { panic!(..) }` or extend the allow. | rust/crates/quoin-modules/src/lag.rs:84-86 |
| FND-002 | low | `catalog validate` propagates a lag-check failure with `?`, so a `modules.list` or deserialisation failure in the warning path turns a passing validation into an error exit, contrary to FR-211 Outputs "Warnings do not change the exit status". | rust/crates/quoin-cli/src/catalog.rs:256 |
| FND-003 | low | A warning is modelled as a `ModulesErrorCode` variant and mapped to `CoreErrorCode::Refused` in the taxonomy; if it ever travels as a `ModulesError` it reports a refusal class for a non-failure. | rust/crates/quoin-modules/src/error.rs:82-84, rust/crates/quoin-core/src/ops/modules/taxonomy.rs:85-86 |
| FND-004 | low | The integration test hard-codes the current iso pin (`"0.27.0"`, `home_with_iso_at("v0.27.0")`), so every future iso pin bump breaks FR-211 tests for no behavioural reason; the pin is already parsed from the manifest in the fixture and should be read from it. | rust/crates/quoin-cli/tests/tc_1839_module_lag.rs:69, rust/crates/quoin-cli/tests/tc_1839_module_lag.rs:76 |
| FND-005 | low | The warning names no remedy, and the CLI exposes only lazy reconcile (`ensure-defaults` skips an installed module), so a user told a module lags has no documented way to bring it to the pin from the message or `module doctor` output. | rust/crates/quoin-cli/src/module.rs:107-110, rust/crates/quoin-modules/src/reconcile.rs:78-81 |

## Verdict

Gate results (locked-build lane 2, shared CARGO_HOME, debug=0): `cargo test -p quoin-modules --lib lag` 3 passed; `-p quoin-cli --test tc_1839_module_lag` 3 passed; `-p quoin-core --lib the_error_mapping` 8 passed; `-p quoin-cli --bin quoin module` 10 passed; `-p quoin-cli --bin quoin help` 5 passed; `-p quoin-modules --lib tc_381_202` FAILED (panicked at error.rs:595, QM032 undocumented); `cargo clippy -p quoin-modules -p quoin-cli --all-targets` exit 0 but 3 warnings in PR code (lag.rs:84-86), all other warnings are in files outside the diff. Running the branch binary against a copy of terra's module home: `module doctor` and `catalog validate` (still `catalog ok (11)`, exit 0) both report spec-artifacts-iso 0.25.0 vs 0.27.0 and engineering-assurance 0.8.2 vs 0.10.1.

Not mergeable as is: FND-001 is a failing existing test, FND-006 fails the `-D warnings` clippy gate, and the spec-side FR id collision (SR-17095) must be fixed in the same round. The core comparison in `lag.rs` is correct and panic-free: it borrows inputs, uses `Option` chaining for absence, strips only a single leading `v`, filters disabled entries, and returns typed `ModuleLag` values. The wire round-trip of `modules.list` back into `ModuleRegistry` works because `InstalledModule` does not deny unknown fields (`pack`, `loadError` are ignored). Diagnostics go to stderr as canonical JSON and do not alter the success outcome of `module doctor`.
