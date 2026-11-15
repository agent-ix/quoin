---
id: SR-20261114002436
title: "Code review of quoin PR 928 (quoin dossier command frame)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@e81a79c8; git diff origin/main...HEAD: rust/crates/quoin-dossier/**, rust/crates/quoin-cli/src/{dossier,main,help,tests}.rs, rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, rust/crates/quoin-core/tests/tc_source_conventions.rs, rust/Cargo.toml, rust/Cargo.lock, rust/crates/quoin-cli/Cargo.toml"
review_set: subset
---

# Code review of quoin PR 928

## Summary

Ticket: PLAT-1614. PR agent-ix/quoin#928 at e81a79c8, scoped to `git diff origin/main...HEAD`
(21 files, +3445/-4). The language-independent code-review sections were run: Duplication and
Vendoring, Ceremony, Integrity, Spec-Code Faithfulness against FR-191 and NFR-040, and Code-Test
Alignment. The Rust lane runs as its own method and has its own artifact
(`plat-1614-rust-review.md`). The gap analysis is `plat-1614-gap-analysis.md`.

No AssuranceProfile applies to `quoin dossier`. No builds or tests were run, by instruction.
`quire matrix` and `quire validate` needed a permission this session did not have, so this
artifact has not been quire-validated.

## Verdict

**CONDITIONAL**: two medium findings and two low findings. Nothing is vendored or copied
between repos. The ignored archetype test reads `SPEC_ARTIFACTS_PROCESS_DIR` in place and copies
nothing. No ledger, pin or file-tracking machinery was added. The `quire` allow-list entry in
`tc_source_conventions.rs` matches the one new executor call site.

On the main path the code does what FR-191 says. Every document is computed in memory before
any write. Writes are staged, can be rolled back, and never clobber a file. Gap order and dossier
order follow the spec. The export argument order and the bundle rule match FR-191 "Commit then
export".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An empty selection is not refused. The shipped build registers no generator (`builtin_generators()` is empty without `test-stubs`), so `quoin dossier generate --version 1` creates an empty `dossier/` directory and exits 0. `quoin dossier export --to pdf --output x.pdf --version 1` finds nothing differing and starts `quire export --to pdf --output x.pdf --source-rev HEAD` with no input file | rust/crates/quoin-dossier/src/engine.rs:94-107, rust/crates/quoin-dossier/src/engine.rs:143-145, rust/crates/quoin-dossier/src/export.rs:60-66, rust/crates/quoin-dossier/src/lib.rs:58-67 |
| FND-002 | medium | The new branch in the shared `emit` prints `payload.rendered` to stdout for every command whose outcome carries no payload (exit 2, 3 or 4), not only for `dossier`. It also ignores `renderedStream`, so a command that returns a stderr-routed `rendered` at exit 2–4 would now write it to stdout. No test covers a non-dossier command on this path | rust/crates/quoin-cli/src/main.rs:558-572 |
| FND-003 | low | `RegistryConflict` maps to exit 4. FR-191's exit table defines 4 only as "`quire export` failed or could not start", so a registry collision reports a status the spec gives another meaning | rust/crates/quoin-dossier/src/error.rs:89 |
| FND-004 | low | Copy-pasted test setup: four near-identical test `Generator` impls (`Doc` in engine.rs, `Doc` in export.rs, `Fixed` in registry.rs, `Fixed` in lib.rs) and two identical `inputs()` builders, all inside one crate's unit-test modules. One shared `#[cfg(test)]` helper would do | rust/crates/quoin-dossier/src/engine.rs:359-406, rust/crates/quoin-dossier/src/export.rs:150-189, rust/crates/quoin-dossier/src/registry.rs:160-182, rust/crates/quoin-dossier/src/lib.rs:81-103 |

## Examined scope

```yaml
clean: false
scope:
  - {id: FR-191, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "Quoin SHALL provide `quoin dossier generate`, which writes the selected dossier documents from the repository's records."}
  - {id: NFR-040, path: spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md, role: examined, excerpt: "`quoin dossier generate` and `quoin dossier check` SHALL give byte-identical Markdown for the same records and options"}
  - {id: rust/crates/quoin-dossier/src/engine.rs, path: rust/crates/quoin-dossier/src/engine.rs, role: examined, excerpt: "render / generate / commit / roll_back / check / ensure_out_dir"}
  - {id: rust/crates/quoin-dossier/src/export.rs, path: rust/crates/quoin-dossier/src/export.rs, role: examined, excerpt: "export / quire_args / run_quire"}
  - {id: rust/crates/quoin-dossier/src/lib.rs, path: rust/crates/quoin-dossier/src/lib.rs, role: examined, excerpt: "builtin_registry / registry_of / builtin_generators"}
  - {id: rust/crates/quoin-dossier/src/registry.rs, path: rust/crates/quoin-dossier/src/registry.rs, role: examined, excerpt: "Registry::register / select"}
  - {id: rust/crates/quoin-dossier/src/error.rs, path: rust/crates/quoin-dossier/src/error.rs, role: examined, excerpt: "DossierErrorCode / DossierError"}
  - {id: rust/crates/quoin-dossier/src/gap.rs, path: rust/crates/quoin-dossier/src/gap.rs, role: examined, excerpt: "GapKind closed list / Gap::order / sort_gaps"}
  - {id: rust/crates/quoin-dossier/src/order.rs, path: rust/crates/quoin-dossier/src/order.rs, role: examined, excerpt: "dossier_cmp / optional_cmp"}
  - {id: rust/crates/quoin-dossier/src/writer.rs, path: rust/crates/quoin-dossier/src/writer.rs, role: examined, excerpt: "compose / push_field"}
  - {id: rust/crates/quoin-dossier/src/report.rs, path: rust/crates/quoin-dossier/src/report.rs, role: examined, excerpt: "Report::outcome / differing"}
  - {id: rust/crates/quoin-dossier/src/stub.rs, path: rust/crates/quoin-dossier/src/stub.rs, role: examined, excerpt: "test-stubs generators"}
  - {id: rust/crates/quoin-cli/src/dossier.rs, path: rust/crates/quoin-cli/src/dossier.rs, role: examined, excerpt: "command / run_with / execute / finish / render_text"}
  - {id: rust/crates/quoin-cli/src/main.rs, path: rust/crates/quoin-cli/src/main.rs, role: examined, excerpt: "usage_refused dossier routing; emit failure_report"}
  - {id: rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs, path: rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs, role: examined, excerpt: "CLI subprocess tests over the test-stubs registry"}
  - {id: rust/crates/quoin-core/tests/tc_source_conventions.rs, path: rust/crates/quoin-core/tests/tc_source_conventions.rs, role: examined, excerpt: "EXPECTED_EXECUTABLES adds quire; KNOWN_ARGUMENTS adds quoin-dossier/src/export.rs"}
  - {id: rust/crates/quoin-dossier/Cargo.toml, path: rust/crates/quoin-dossier/Cargo.toml, role: examined, excerpt: "test-stubs = [] feature; workspace lints"}
  - {id: rust/crates/quoin-cli/Cargo.toml, path: rust/crates/quoin-cli/Cargo.toml, role: examined, excerpt: "quoin-dossier dev-dependency with features = [\"test-stubs\"]"}
```

## Dispositions

Round 1, verified against fix commit 481d46a9 (`git diff e81a79c8..481d46a9`). No builds or tests were run, by instruction.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

- FND-001 after_excerpt (`rust/crates/quoin-dossier/src/registry.rs:119-125`): `if self.generators.is_empty() { return Err(DossierError::new(DossierErrorCode::NoDocuments, "no dossier documents are registered in this build, so there is nothing to select; ...")); }`. `render` calls `select` before `ensure_out_dir` and before `run_quire`, so `generate` creates no directory and `export` never starts quire. `NoDocuments` maps to exit 2. Tests: `engine::an_empty_registry_is_refused_before_a_directory_is_created`, `registry::an_empty_registry_refuses_every_selection_with_exit_2`, and `lib::the_shipped_registry_is_empty_and_every_run_is_refused_without_writing`.
- FND-002 after_excerpt (`rust/crates/quoin-cli/src/main.rs:570`): `if response.outcome.carries_payload() || is_failure_report(&response.payload) {`. The report is printed only when the payload carries `failureReport: true`, which only `dossier::report_response` sets. The output now goes through the existing `renderedStream` branch, so stream routing is honoured. Tests: `tests::only_a_marked_response_prints_its_report_at_a_failure_status` and `tc_1614::a_failure_of_another_command_prints_nothing_on_stdout`.
- FND-003 after_excerpt (`rust/crates/quoin-dossier/src/error.rs:99-102`): `| Self::RegistryConflict | Self::NoDocuments => Outcome::Refused, Self::SourceFailed => Outcome::Invalid, Self::QuireFailed => Outcome::Internal,`. `codes_map_to_the_exit_table` asserts `RegistryConflict` is exit 2.
- FND-004 after_excerpt (`rust/crates/quoin-dossier/src/testing.rs:23-27`): `pub(crate) struct Doc { pub(crate) spec: DocumentSpec, pub(crate) gaps: fn() -> Vec<Gap>, pub(crate) fail: bool, }`. This is one `#[cfg(test)] mod testing` holding `doc`, `inputs`, `registry` and `names`. The engine, export, registry and lib test modules import it, and their local `Doc`/`Fixed`/`inputs()` copies are gone.
