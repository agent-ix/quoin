---
id: SR-20261112024905
title: "Code review of quoin PR 829 (FR-136 quoin anomaly)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@91a5f857; rust/crates/quoin-anomaly/**, rust/crates/quoin-cli/src/anomaly.rs, rust/crates/quoin-cli/src/{help,main,tests}.rs, rust/crates/quoin-cli/tests/{tc_136_anomaly_command,tc_shell_contract}.rs, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, spec/assurance/MP-250-nfr003-actionable-failures.md, spec/assurance/MP-251-nfr003-silent-failures.md, rust/Cargo.toml, rust/Cargo.lock"
review_set: subset
---

# Code review of quoin PR 829

## Summary

Ticket: PLAT-1415 (with PLAT-1416). This review covers the new `quoin-anomaly` crate and the `quoin anomaly check` and `quoin anomaly refs` commands, with the Rust lane (rust-style) folded in. Diff: `git diff origin/main...HEAD` at 91a5f857. Gates run: `cargo fmt --all --check` passed. `cargo clippy -p quoin-anomaly -p quoin-cli --all-targets --all-features -D warnings` was clean. `cargo test -p quoin-anomaly` passed 13 tests. The quoin-cli tests `tc_136_anomaly_command`, `tc_shell_contract` and the bin unit tests passed 2, 3 and 60. These tests ran with `SPEC_ARTIFACTS_PROCESS_DIR` pointed at a checkout of spec-artifacts-process#140. The built binary was also run against scratch git repositories.

## Verdict

**FAIL: two high findings.** Two of them were reproduced against the binary:

- History lookup returns nothing for any anomaly that is not at the repository root, so in a real `spec/...` layout an illegal transition passes.
- The `line` of a no-record document is the line of the mapping error, not the line of the `status` field.

What is right: the reachability table matches the legal moves of spec-artifacts-process FR-014 exactly. The letter-to-signature mapping is right. The pending working-tree entry is right. Rename-following worked in a probe. The refs ordering and the empty-array JSON are right. The command is registered in the help, the route census and the unknown-command catalogue.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `read_history` runs `git -C <file's parent dir> log ... -- <path relative to the toplevel>`. Git resolves a pathspec relative to the cwd, so for any file below the root it looks for `spec/anomaly/spec/anomaly/AN-001.md` and finds nothing. History is empty and no transition is judged. Reproduced: `spec/anomaly/AN-001.md` committed `closed` with `assessed` in the working tree gives no illegal-transition, while the same file at the root does. Every test puts the file at the repository root | rust/crates/quoin-anomaly/src/history.rs:127-155 |
| FND-002 | high | When parsing fails, every diagnostic of the document takes `error.line`, the line of the mapping error (for example the table header), not the frontmatter `status` line that FR-136 Diagnostics and AC-11 require. Reproduced: a `columns` error reported both `anomaly.no-record` and `anomaly.illegal-transition` at line 16, with `status` on line 5 | rust/crates/quoin-anomaly/src/check.rs:193-196 |
| FND-003 | medium | Git failures are swallowed. `git_text` maps a spawn failure to `None`, so with no `git` on PATH every file is `NotApplicable` and the transition checks are skipped silently (reproduced). A failing `git log` becomes an empty history through `unwrap_or_default()`. `read_history` can never return `Err`, so its `# Errors` doc ("Returns an error when git cannot be run") is false | rust/crates/quoin-anomaly/src/history.rs:87-93,128,155 |
| FND-004 | medium | The record reader repeats the module mapping, which FR-136 says Quoin repeats none of. The vocabularies are hard-coded (`COLUMNS`, `KINDS`, `IMPACTS`) and the error codes are invented (`missing`, `columns`, `row-arity`, `column-choices`, `min-rows`). The reader is about 230 lines and the code does not mark it as a stand-in for quire-rs#393 with a removal condition. Effect: an out-of-vocabulary Impact, which FR-014's owner table gives to the manifest and the `assert` facet (quire validate), is reported as `anomaly.no-record (column-choices)`. That is a field diagnostic quoin SHALL NOT repeat. The document also drops out of refs and missing-module. Reproduced on the `invalid/impact-outside-vocabulary` fixture | rust/crates/quoin-anomaly/src/record.rs:4-17,142-176 |
| FND-005 | medium | Module discovery is hand-rolled and duplicates the discovery in quoin-core `HostCatalog` (runtime.rs:84-160), diverging from it: `QUOIN_MODULE_PATHS` is split on `:` instead of `split_paths`, and a candidate that is a directory of module roots is not searched. A `--module` path that does not exist is silently accepted and produces a false `anomaly.missing-module` (an NFR-003 silent failure; reproduced with `--module /nonexistent`) | rust/crates/quoin-cli/src/anomaly.rs:128-169 |
| FND-006 | medium | The NFR-003 populations are incomplete. MP-250 and MP-251 enumerate every command crossed with each applicable class, yet only `anomaly:unknown-flag` was added. The family pattern also needs `anomaly:unknown-subcommand`, `anomaly check:unknown-flag`, `anomaly refs:unknown-flag`, `anomaly refs:missing-argument`, `anomaly check:missing-path`, `anomaly check --module:missing-path` and `anomaly refs --scope:missing-path` | spec/assurance/MP-250-nfr003-actionable-failures.md:21; spec/assurance/MP-251-nfr003-silent-failures.md:21 |
| FND-007 | medium | The tests panic unless a sibling spec-artifacts-process checkout holds `tests/fixtures/anomaly`. The local sibling checkout on main lacks it today, since spec-artifacts-process#140 is unmerged. A clean clone fails too, and so does the CI workflow, which checks out only quoin. `cargo test --workspace` (make test) goes red until #140 merges and every sibling checkout is pulled. Reading in place is not vendoring: the tempdir copies are made at runtime and there is no repo copy. But the lane is undefined: there is no checkout step, no pinned source and no skip-with-reason | rust/crates/quoin-anomaly/tests/tc_136_anomaly.rs:35-47; rust/crates/quoin-cli/tests/tc_136_anomaly_command.rs:22-34 |
| FND-008 | low | The CLI prints the literal `error` instead of `diagnostic.severity`, drops `commits` from the output and leaves `Response.diagnostics` empty. A missing scope also exits 1, the same status as "diagnostics found", so an operational failure cannot be told apart from findings by exit code | rust/crates/quoin-cli/src/anomaly.rs:78-109 |
| FND-009 | low | Stringly typed fields (rust-style: Copy code enums, closed enums). `StatusEntry.status` and `Record.status` are `String` although `Status` exists. `MappingError.code` is a `&'static str`, not a Copy enum, and the tests branch on the text `"columns"`. `Diagnostic.reason` is a `String` that repeats `code.reason()` | rust/crates/quoin-anomaly/src/record.rs:56-63; rust/crates/quoin-anomaly/src/history.rs:47; rust/crates/quoin-anomaly/src/check.rs:71 |

## Examined scope

```yaml
clean: false
scope:
- {id: quoin-anomaly/src/status.rs, path: rust/crates/quoin-anomaly/src/status.rs, role: examined, excerpt: "MOVES table and Status::reaches; matches spec-artifacts-process FR-014 legal moves"}
- {id: quoin-anomaly/src/history.rs, path: rust/crates/quoin-anomaly/src/history.rs, role: examined, excerpt: "read_history, default_branch, Signature::from_code"}
- {id: quoin-anomaly/src/check.rs, path: rust/crates/quoin-anomaly/src/check.rs, role: examined, excerpt: "check_document, transition_diagnostics"}
- {id: quoin-anomaly/src/record.rs, path: rust/crates/quoin-anomaly/src/record.rs, role: examined, excerpt: "parse_record, rows, status_word, assessment_text"}
- {id: quoin-anomaly/src/refs.rs, path: rust/crates/quoin-anomaly/src/refs.rs, role: examined, excerpt: "anomaly_documents, resolvable_ids, references_to"}
- {id: quoin-anomaly/src/error.rs, path: rust/crates/quoin-anomaly/src/error.rs, role: examined, excerpt: "ErrorCode + AnomalyError envelope"}
- {id: quoin-cli/src/anomaly.rs, path: rust/crates/quoin-cli/src/anomaly.rs, role: examined, excerpt: "command, run_check, run_refs, default_roots, declares_hazard"}
- {id: quoin-cli registration, path: rust/crates/quoin-cli/src/help.rs, role: examined, excerpt: "COMMANDS entry, unknown_command catalogue, NATIVE_OWNED_ROUTES, retained-command-help.json"}
- {id: MP-250/MP-251, path: spec/assurance/MP-250-nfr003-actionable-failures.md, role: examined, excerpt: "population_set adds anomaly:unknown-flag"}
```
