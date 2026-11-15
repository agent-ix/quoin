---
id: SR-20261114002437
title: "Rust review of quoin PR 928 (quoin-dossier crate and dossier CLI module)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@e81a79c8; git diff origin/main...HEAD: rust/crates/quoin-dossier/**, rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-cli/src/main.rs, rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs, rust/Cargo.toml, rust/crates/quoin-cli/Cargo.toml"
review_set: subset
---

# Rust review of quoin PR 928

## Summary

Ticket: PLAT-1614. This is the `rust-review` method, run on its own because the brief asked for
one record per method. Its findings are not repeated in the code-review artifact
(`plat-1614-code-review.md`). It applies the repository idiom doc
`.claude/skills/rust-style/SKILL.md`, which outranks the ecosystem skill, plus the ecosystem
`rust-review` core checks.

**Gates were not run.** The brief said no builds or tests, so `fmt --check`, `clippy -D
warnings`, `cargo test` and `cargo deny` were not run. The PR body reports focused fmt, clippy
and test runs, but those are the author's claims and were not re-measured here. The PR changes
no CI workflow file.

## Verdict

**CONDITIONAL**: three low findings and nothing medium or high. The crate follows the repository
idioms:

- **Headers and lints:** every new `.rs` file has the AGPL-3.0-or-later SPDX header, and the
  crate opts into `[lints] workspace = true`.
- **Error envelope:** `DossierErrorCode` follows the envelope pattern (`as_str`, `all`,
  `from_code`, `outcome`, a round-trip uniqueness test, `Box<str>` message, `BTreeMap`
  context).
- **Panics:** production code has no `unwrap`/`expect`/indexing. Every test module carries
  exactly the sanctioned `#[allow]` set with its `reason`.
- **Trace tags:** tests use `#[trace(...)]` with `/// Provenance:` lines.
- **Determinism:** no wall-clock or network crate is in the dossier path.
- **Staging:** writes use exclusive `tempfile` files and `persist_noclobber`.
- **Test stubs:** `test-stubs` reaches the binary only through the `quoin-cli` dev-dependency.
  Under resolver 3, `cargo build --release -p quoin-cli --bin quoin` (the release command) does
  not enable it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `quoin-cli` creates the diagnostic code `dossier.export_stale` as a string literal outside `DossierErrorCode`. So `all()`, `from_code()` and the round-trip/uniqueness test do not cover it, and the dossier surface now has two code catalogues with two spellings (`DOSSIER_*` and `dossier.*`). This breaks the rust-style "Error envelopes" rule that codes are the API and live in the one code enum | rust/crates/quoin-cli/src/dossier.rs:268-276 |
| FND-002 | low | `let _ = fs::remove_file(&options.output)` discards the error. If quire fails and the partial output cannot be removed, FR-191-AC-7's "leaves no output file" is silently broken and the report says nothing | rust/crates/quoin-dossier/src/export.rs:128-132 |
| FND-003 | low | Every module is `pub mod` (`engine`, `export`, `gap`, `order`, `registry`, `report`, `writer`), and `Rendered` (all fields public), `report_with_comparison`, `quire_args`, `dossier_cmp` and `optional_cmp` are public. Yet `quoin-cli`, the only consumer, uses only the root re-exports. That is public exposure with no consumer or stated contract (rust-style: `pub` only where a consumer reaches) | rust/crates/quoin-dossier/src/lib.rs:13-29, rust/crates/quoin-dossier/src/engine.rs:25-35, rust/crates/quoin-dossier/src/export.rs:69-71 |

## Examined scope

```yaml
clean: false
scope:
  - {id: rust/crates/quoin-dossier/src/engine.rs, path: rust/crates/quoin-dossier/src/engine.rs, role: examined, excerpt: "commit(): preflight symlink_metadata, staged tempfile, backup rename, persist_noclobber, roll_back"}
  - {id: rust/crates/quoin-dossier/src/export.rs, path: rust/crates/quoin-dossier/src/export.rs, role: examined, excerpt: "run_quire: Command::new(&options.quire).args(quire_args(..)).stdin(Stdio::null()).output()"}
  - {id: rust/crates/quoin-dossier/src/error.rs, path: rust/crates/quoin-dossier/src/error.rs, role: examined, excerpt: "DossierErrorCode as_str/all/from_code/outcome"}
  - {id: rust/crates/quoin-dossier/src/lib.rs, path: rust/crates/quoin-dossier/src/lib.rs, role: examined, excerpt: "pub mod engine; pub mod error; pub mod export; pub mod gap; pub mod order; pub mod registry; pub mod report; pub mod writer;"}
  - {id: rust/crates/quoin-dossier/src/registry.rs, path: rust/crates/quoin-dossier/src/registry.rs, role: examined, excerpt: "pub trait Generator: Send + Sync"}
  - {id: rust/crates/quoin-dossier/src/gap.rs, path: rust/crates/quoin-dossier/src/gap.rs, role: examined, excerpt: "GapKind::position(self) -> usize { self as usize }"}
  - {id: rust/crates/quoin-dossier/src/order.rs, path: rust/crates/quoin-dossier/src/order.rs, role: examined, excerpt: "segment_cmp compares digit strings without numeric parse"}
  - {id: rust/crates/quoin-dossier/src/writer.rs, path: rust/crates/quoin-dossier/src/writer.rs, role: examined, excerpt: "push_field quotes every value with serde_json::to_string"}
  - {id: rust/crates/quoin-dossier/src/report.rs, path: rust/crates/quoin-dossier/src/report.rs, role: examined, excerpt: "Report::outcome Ok/Partial"}
  - {id: rust/crates/quoin-dossier/src/stub.rs, path: rust/crates/quoin-dossier/src/stub.rs, role: examined, excerpt: "#[cfg(feature = \"test-stubs\")] mod stub"}
  - {id: rust/crates/quoin-dossier/Cargo.toml, path: rust/crates/quoin-dossier/Cargo.toml, role: examined, excerpt: "[lints] workspace = true; test-stubs = []"}
  - {id: rust/crates/quoin-cli/src/dossier.rs, path: rust/crates/quoin-cli/src/dossier.rs, role: examined, excerpt: "finish(): diagnostics, report_response, export_stale"}
  - {id: rust/crates/quoin-cli/src/main.rs, path: rust/crates/quoin-cli/src/main.rs, role: context_only, excerpt: "emit(): failure_report branch (reported under code-review FND-002)"}
  - {id: rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs, path: rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs, role: examined, excerpt: "#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, reason = ...)]"}
  - {id: .claude/skills/rust-style/SKILL.md, path: .claude/skills/rust-style/SKILL.md, role: context_only, excerpt: "Codes are the API. Never rename one, never reuse one."}
```

## Dispositions

Round 1, verified against fix commit 481d46a9 (`git diff e81a79c8..481d46a9`). No builds or tests were run, by instruction.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

- FND-001 after_excerpt (`rust/crates/quoin-cli/src/dossier.rs:275`): `diagnostics.push(diagnostic(&DossierError::export_stale(id)));`. `DossierErrorCode::ExportStale` (`DOSSIER_EXPORT_STALE`, exit 1) is now in `as_str`, `all()` and `outcome()` (`error.rs:43,61,79,103`), so the round-trip and uniqueness test covers it. No `dossier.export_stale` literal remains.
- FND-002 after_excerpt (`rust/crates/quoin-dossier/src/export.rs:137-148`): `match fs::remove_file(&options.output) { Ok(()) => {} Err(error) if error.kind() == std::io::ErrorKind::NotFound => {} Err(error) => { ... write!(failure.stderr, "could not remove the partial output {}: {error}", options.output.display()); } }`. Tests: `a_partial_output_from_a_failed_quire_is_removed`, `a_failed_quire_that_wrote_nothing_is_not_a_cleanup_error`, `an_output_that_existed_before_a_failed_quire_is_preserved` and `a_partial_output_that_cannot_be_removed_is_reported_beside_quires_stderr`.
- FND-003 after_excerpt (`rust/crates/quoin-dossier/src/lib.rs:13-24`): `mod engine; mod error; mod export; mod gap; mod order; mod registry; mod report; #[cfg(feature = "test-stubs")] mod stub; #[cfg(test)] mod testing; mod writer;`. `Rendered` and its fields, `render`, `report_with_comparison`, `quire_args`, `dossier_cmp`, `optional_cmp`, `BUNDLE_*`, `Registry::select`/`specs` and the `Gap` helpers are now `pub(crate)`. Only the root `pub use` items remain public, and those are the items `quoin-cli` uses.
