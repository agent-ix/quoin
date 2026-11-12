---
id: SR-20261112191017
title: "Code review of quoin PR 831 (licensed pack detection, hosted-source refusals, sandbox harness)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@5cd35f5c; rust/Cargo.toml, rust/Cargo.lock, rust/crates/quoin-quire/src/{clauses,engine,lib}.rs, rust/crates/quoin-modules/src/{pack,install,error,lib}.rs, rust/crates/quoin-core/src/{runtime,capabilities}.rs, rust/crates/quoin-core/src/ops/modules/*, rust/crates/quoin-cli/src/module.rs, rust/crates/quoin-cli/tests/{pack_support/mod.rs,tc_1492_licensed_packs.rs,tc_1493_pack_sandbox.rs}, rust/crates/quoin-modules/tests/tc_1492_packs.rs"
review_set: subset
---

# Code review of quoin PR 831

## Summary

Ticket: PLAT-1492 (and PLAT-1493). The Rust review lane (rust-review with the repo's
`.claude/skills/rust-style`) is folded in. Tools: quoin 0.30.1. Focused tests, one target dir via
locked-build.sh (deleted after): quoin-modules lib + tc_1492_packs (35 pass), quoin-quire lib
(tc_1492_001..004 pass), quoin-cli tc_1492_licensed_packs (9) and tc_1493_pack_sandbox (3) pass,
quoin-core `modules` lib tests (19) pass.

## Verdict

Correct on the main paths. The rights decision is only `quire_rs::check_publishable` through
`quoin_quire::clauses::restricted_clause_sets`; a scan of non-test source finds no `TextRights`,
`Licensed` or `clearance_ref`. The quire-rs bump is `=0.59.0` plus a rev equal to the v0.59.0 tag
object, the same form as the 0.56.0 pin it replaces, with only `quoin-yaml` added to the lock.
The `.git` walk uses `symlink_metadata` on every ancestor (dir, file, stray entry), the source is
canonicalized in `resolve()` (fails closed as `PathSourceNotFound`), and the refusal runs before
the registry snapshot and `materialize`. A load failure is rolled back after the gate. `list`
maps each row to `pack: true|false|null` with `loadError` and never fails as a whole.
`ensure-defaults` refuses through `Origin::Default`. The `publish_check` token and fn-pointer
witness follow the `clause_sets` pattern. The FR-139 control case does fail the oracle, and the
sentinel search covers the registry. The help fixture is unchanged and needs no change. Four
defects, all in the tests or a redundant fallback.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `semantic_root()` materializes the contract under `std::env::temp_dir()` (`/tmp`, which on this machine sits inside a `/tmp/.git` work tree) as `quoin-cli-packs-semantic-<pid>` and never removes it. Every test-binary run leaks one directory outside the scratch dirs (10+ observed after one run). Materialize it under `clean_base()` in a `TempDir` the test owns, or per sandbox | rust/crates/quoin-cli/tests/pack_support/mod.rs:73-81 |
| FND-002 | medium | Several refusal tests assert only a non-zero exit, so any failure passes them. These are tc_1492_108 (the only test for FR-137-AC-10's gitignore and GIT_DIR/GIT_WORK_TREE), tc_1492_103 (AC-4 through the binary) and the refused leg of tc_1492_107 (AC-9). Assert `QM024_PACK_INSIDE_WORK_TREE` and the work-tree root in the output | rust/crates/quoin-cli/tests/tc_1492_licensed_packs.rs:123,205,233 |
| FND-003 | low | tc_1492_104 checks the recomputed verdict with `!listed.contains("\"pack\": true")`. That also passes when the edited module fails to load (`"pack": null`). Assert the widget-pack row reads `"pack": false` | rust/crates/quoin-cli/tests/tc_1492_licensed_packs.rs:161 |
| FND-004 | low | `content_root.canonicalize().unwrap_or_else(\|_\| content_root.to_path_buf())` falls back to the uncanonical path on error. For an ad-hoc `Path` source, `content_root` is already canonical because `resolve()` canonicalizes and fails closed, so this call is redundant and fails open in form. Judge `content_root` directly | rust/crates/quoin-modules/src/install.rs:236-238 |
