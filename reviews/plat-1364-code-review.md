---
id: SR-16393
title: "Code review of quoin PR 777 (semantic-module fixtures from quire-fixtures)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@a4ffd114; rust/Cargo.toml, rust/Cargo.lock, rust/crates/{quoin-cli,quoin-core,quoin-modules,quoin-semantic}/Cargo.toml, rust/crates/quoin-cli/tests/tc_module_command.rs, rust/crates/quoin-core/tests/{tc_446_reconcile_revalidation,tc_452_module_install_criteria,tc_452_semantic_boundary}.rs, rust/crates/quoin-modules/src/contract_gate.rs, rust/crates/quoin-semantic/tests/{common/mod.rs,tc_452_manifest_criteria.rs,tc_452_sweep_criteria.rs}, tests/fixtures/semantic-module/ (deleted)"
review_set: subset
---

# Code review of quoin PR 777

## Summary

Ticket: PLAT-1364 (child of PLAT-287). Diff `origin/main...HEAD` at `a4ffd114`. The PR deletes
`tests/fixtures/semantic-module/` and points every reader at the `quire-fixtures` crate of
agent-ix/quire-rs, added as a workspace git dependency at quoin's existing quire-rs rev
`ce172a7` and used only from `[dev-dependencies]`. Rust lane (rust-review, rust-style loaded)
folded in.

Checks run:

- Byte comparison of every deleted file against `quire-fixtures` at `ce172a7`: 10 files
  identical; the rest differ because the authoritative copy moved on (Entity.json `$id` now
  carries `/0.1.0/` and refs semantic-core `0.3.3`; `manifest.yaml` drops `semantic_core`;
  `legacy-free-column-table.md` became the smaller `legacy-free-table.md`).
  `clause-language-0.1.0-cases.json` has no upstream counterpart. No quoin test or spec reads it,
  and it belonged to TC-1716, which is withdrawn, so dropping it is correct.
- No copy between repos: the tests copy `module-ok` from cargo's git checkout into a scratch dir
  when they run. Nothing is committed.
- The `semantic_core: 0.0.0` injection: quoin builds its contract from npm
  `@agent-ix/semantic-schema` 0.3.0 (released at filament-core-data `6156f9e`), and its
  module-manifest schema requires `semantic.semantic_core`. PLAT-1221 (Done) removed that
  requirement in filament-core-data `dd9ea99` (2026-10-30), and `quire-fixtures` followed it, but
  no semantic-schema release carries it yet. The injection works around quoin's older pinned
  contract and hides no new incompatibility. It restates the accommodation the deleted quoin
  fixture already carried inline ("BLOCKED-BY PLAT-1221"). The fix is to bump quoin's
  semantic-schema dependency, not to change quire-fixtures (see FND-002).
- Production leak: `cargo tree -e normal,build -i quire-fixtures` prints nothing. The crate is
  dev-only, and the workspace is `publish = false`.
- Focused tests, run against a node_modules built from the repo's pinned versions
  (semantic-schema 0.3.0 and semantic-core 0.3.2, both from filament-core-data `6156f9e`). The
  coder's symlink to `/home/peter/dev/quoin/node_modules` holds stale versions (semantic-schema
  0.1.0, semantic-core 0.3.0). Results: tc_module_command 2, tc_446 3, tc_452_module_install 4,
  tc_452_semantic_boundary 5, tc_452_manifest_criteria 14, tc_452_sweep_criteria 7, and the
  quoin-modules `contract_gate` unit tests 10. All pass.
- Vacuity: `tc_452_654` keeps its floors (5 or more cases, 3 or more forms). The sweep corpus
  copies five named files with `fs::copy(..).unwrap()`, so a missing file panics. No loop runs
  over a directory listing.
- Lock churn: re-resolving `origin/main`'s Cargo.lock with only this PR's manifests
  (`cargo metadata --offline`) produces a lock byte-identical to the PR's. Cargo itself moved
  `agent-ix-semantic-schema` and switched the windows-sys edges (see FND-005).

## Verdict

**PASS WITH FINDINGS (all low).** The de-vendoring is real. The authoritative fixtures are read
through a dev-only dependency, nothing is copied into the repo, and the affected suites pass
against the pinned contract. The findings are maintainability and comment accuracy.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `fixture()` helper (copy `module-ok` to `temp_dir()/<name>-<pid>`, then inject `semantic_core: 0.0.0` by string replace) is written out six times. Each copy hard-codes the literal `0.0.0` instead of `common::SCHEMA_REQUIRED_SEMANTIC_CORE`, so removing the accommodation later means finding six sites | rust/crates/quoin-cli/tests/tc_module_command.rs:28-45; rust/crates/quoin-core/tests/tc_446_reconcile_revalidation.rs:85-102; rust/crates/quoin-core/tests/tc_452_module_install_criteria.rs:103-121; rust/crates/quoin-core/tests/tc_452_semantic_boundary.rs:131-162; rust/crates/quoin-modules/src/contract_gate.rs:420-440; rust/crates/quoin-semantic/tests/common/mod.rs:78-96 |
| FND-002 | low | The new doc comments call `semantic_core` something the fixture "does not carry", with no expiry. The old fixture tied it to PLAT-1221. The real expiry is quoin bumping `@agent-ix/semantic-schema` past 0.3.0 to a release that includes filament-core-data dd9ea99. The PR description's "clean fix upstream in quire-fixtures" is the wrong direction: re-adding the key there would regress PLAT-1221 | rust/crates/quoin-core/tests/tc_446_reconcile_revalidation.rs:85-86 (and the five sibling helpers) |
| FND-003 | low | The scratch copies at `std::env::temp_dir().join("<name>-<pid>")` are never removed. Every test process leaves one in /tmp; ten were present after the coder's and this reviewer's runs. The pattern copies the existing `semantic_root()` helpers, which leak the same way, so this is a convention to fix once, not a regression unique to this PR | rust/crates/quoin-semantic/tests/common/mod.rs:82-84 |
| FND-004 | low | The tc_452_616 `$id` check was loosened to `contains("/spec-objects-fixture/") && ends_with("/Entity.json")`. It still rejects any other bundle document, but it could assert equality with the authoritative file's own `$id`, read from `quire_fixtures::module_ok_dir()`, and keep the earlier exactness | rust/crates/quoin-semantic/tests/tc_452_manifest_criteria.rs:274-280 |
| FND-005 | low | Cargo.lock moves `agent-ix-semantic-schema` (branch=main) from 45a927b to d694d7f, and the windows-sys edges change from 0.61.2 to 0.52.0. Cargo produces both on any re-resolution. The filament commits differ only in four test files, so there is no functional change. `cargo update -p agent-ix-semantic-schema --precise 45a927b258c297159567e4be6a40b6f466a42516` reverts that part and stays stable. The windows-sys edges affect Windows targets only | rust/Cargo.lock:22; rust/Cargo.lock:549 |
