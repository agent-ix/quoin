---
id: SR-1854004
title: "PLAT-1854 quoin PR 1039 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PR 1039 (rust/Cargo.lock, rust/crates/quoin-quire/{Cargo.toml,src/{error,coverage,design_join,design_matrix,evidence,flow_report,assurance,modules,lib}.rs,tests/fr029_dropped_declarations.rs}, rust/crates/quoin-core/src/{protocol.rs,ops/quire/{mod,taxonomy}.rs}, rust/crates/quoin-cli/src/{core_runtime,analysis_evidence,assurance,dossier,formal_rules,formal_rules_coverage,matrix}.rs, rust/crates/quoin-cli/tests/tc_1854_dropped_declarations_{a,b}.rs, rust/crates/quoin-soup/src/soup.rs, rust/crates/quoin-suite-trace/{Cargo.toml,src/{main,error}.rs,tests/tc_1854_dropped_declarations.rs}, rust/crates/quoin-dossier/Cargo.toml)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-029"
    type: references
---

# PLAT-1854 quoin PR 1039 code-review

## Summary

Ticket: PLAT-1854. Code review with the Rust lane (rust-review, ecosystem rust-style and the repository's own rust-style skill) folded in, scoped to `git diff origin/main...HEAD` at head 7f9ab78c (merge base a449fa31). Tool versions: quoin 0.36.8, module spec-artifacts-process@v0.36.0, quire-rs linked at b6beef4 (package 0.62.3).

The typed error, its code QQ-1022, the `Refused` class, the `quire_code` and `dropped_declarations` envelope keys, the exhaustive `coverage_error` match and the relation-report mapping all match FR-029. Every caller of `registry.traceability()` in production code was measured (coverage walk, `test_traces`, evidence `compute_with`, assurance `build`, formal-rules `status` and `assurance_declarations`), and each now asks `require_complete_model` before reading the model and before the absent-model answer. Every CLI path that reaches `quire.coverage`, `quire.design_matrix` or `quire.design_join` (advise, anomaly check, evidence record and audit, class obligations, matrix, assurance, formal-rules coverage, soup promote, release snapshot, dossier sources) tests `carries_payload()` before reading, so the new `invoke` refusal response cannot be read as a payload. Paths that stringify the error keep the code, because the `Display` of the variant starts with `QQ-1022`.

Mutation results (each mutation applied alone, focused tests run, tree restored): removing the check from the coverage walk, from `test_traces`, from `assurance::build`, from formal-rules `status`, from `assurance_declarations`, from the dossier up-front check, from soup `invoke` or from CLI `invoke`, and mapping the suite-trace refusal to an engine error, each fail at least one test. Mapping a relation-report drop to QQ-1110, and mapping a coverage drop to QQ-1020, fail the unit tests. Removing the check from evidence `compute_with` or from `flow_report::compute` survives. Both are equivalent mutants: the evidence compute reads the registry the walk has just checked, and the engine's relation report refuses a drop by itself. Moving the check after the absent-model answer survives in `test_traces` and in formal-rules `status`; those test gaps are recorded in the gap-analysis review SR-1854005.

Pre-existing (not caused by this PR): `tc_1650_native_shell_route_census_matches_the_retained_owned_surface` fails because PLAT-1839 (#1027) added `module doctor` and `plugin doctor` routes without updating `NATIVE_OWNED_ROUTES`. This PR does not touch `main.rs`, `tests.rs` or any command definition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `quire.properties` operation computes over the merged traceability model and does not refuse a drop. `quire_rs::classify_document_criteria` reads `registry.traceability()` to attach each criterion's obligation (FR-053), and `properties::classify` never calls `require_complete_model`. A dropped `obligations:` source silently returns criteria with no obligation. In `quoin advise` the coverage refusal runs first, but advise sends `--module` to coverage and `modules: []` (scope or ambient) to properties, so a drop in the ambient set is computed over whenever `--module` is given. FR-029 says every operation that computes over the merged model refuses. Add the check to `properties::classify`. | rust/crates/quoin-quire/src/properties.rs:421-476; rust/crates/quoin-cli/src/advise.rs:34-46 |
| FND-002 | medium | Pre-existing on origin/main, but it makes FR-029-AC-23 unreachable for `quoin formal-rules coverage`. Every clean scope that holds a criterion exits 3 with "criterion FR-001-AC-1 has missing or malformed source document metadata". The cause is the projection, not this PR: `quoin_quire_types::Obligation` has no `document` field, so `quoin-core`'s `fn obligation` drops the engine's `Obligation.document`, and `criterion_source_items` requires it on every criterion row. This PR's change to `ops/quire/mod.rs` only re-exports. Fix the projection, here or under its own ticket, then add the AC-23 clean-run test for formal-rules coverage. | rust/crates/quoin-core/src/ops/quire/mod.rs:260-270; rust/crates/quoin-quire-types/src/lib.rs:82-99; rust/crates/quoin-cli/src/formal_rules_coverage.rs:418-427 |
| FND-003 | low | The dossier up-front check refuses every `dossier generate`, `check` and `export` when any declaration dropped, including document kinds that read no model (the test refuses `--doc stub-a`). The doc comment says "a document kind that reads no model still generates", which is true only for other module-resolution failures. FR-029 names the dossier commands, so refusing is acceptable, but the comment misstates it. | rust/crates/quoin-cli/src/dossier.rs:185-215 |
| FND-004 | low | The quire-rs pin is `rev = b6beef4`, which is quire-rs `main` 12 commits after tag `v0.62.3`, yet the version is declared `=0.62.3`, the same version as the released tag at a different commit. The comment still calls it "THE RELEASE CONTRACT". The bump also brings a real code change in quire-code-parse (0.2.2 at 108ddda to 0.2.3 at tag v0.2.3: `parse.rs`, `language.rs`, `error.rs`). The `agent-ix-semantic-schema` source move (branch=main c23980e to tag semantic-schema-v0.6.0 327a49c) is byte-identical crate content, and the `windows-sys` 0.52 to 0.61.2 edge moves are a Windows-only re-resolve. Before a quoin release, move both manifests to a quire-rs tag that contains #715, and reword the comment. | rust/crates/quoin-quire/Cargo.toml:21-41; rust/crates/quoin-dossier/Cargo.toml:42; rust/Cargo.lock:3070-3102 |
| FND-005 | low | `Response::refusal` is the payload-free refusal response that `invoke_refusable` and `invoke_declining` (and `invoke_verifying` on current main) still build inline. Use the new constructor in those places too, so the shape is defined once. | rust/crates/quoin-core/src/protocol.rs:119-131; rust/crates/quoin-cli/src/core_runtime.rs:97-130 |

## Gates

The focused tests were run through the shared locked-build lane (debug=0, incremental=0, shared CARGO_HOME) at 7f9ab78c, reusing the worktree's `rust/target`. All passed: `cargo test -p quoin-cli --test tc_1854_dropped_declarations_a` (12 passed), `--test tc_1854_dropped_declarations_b` (13 passed), `cargo test -p quoin-quire --test fr029_dropped_declarations` (8 passed), quoin-quire unit filters (3 passed), quoin-core taxonomy filters (9 passed), the quoin-cli bin filter `the_assurance_declaration_read_refuses` (1 passed), and `cargo test -p quoin-suite-trace --test tc_1854_dropped_declarations` (2 passed). The worktree had no `node_modules`, so `pnpm install --frozen-lockfile` was run against npmjs before building. Clippy was not run: the brief marks the pre-existing clippy errors out of scope, and no new `#[allow]` without a reason appears in the diff.

Merge overlap: origin/main has moved on to 61c4dfdc (#1034 PLAT-1805, #1035, #1037 and #1038 are merged). A three-way `git merge-tree` shows changes on both sides in `Cargo.lock`, `quoin-cli/src/{assurance,core_runtime,dossier,formal_rules}.rs` and `quoin-quire/src/modules.rs`, with no textual conflict. No new `registry.traceability()`, `quire.coverage`, `quire.design_*`, flow-report, evidence or trace-tag caller landed on main since the merge base. Rebase, then rebuild the focused tests once.

## Verdict

CHANGES REQUESTED (FND-001, FND-002). The refusal is wired through every measured path, its code and envelope match FR-029, and the CLI tests check exit 2, `QQ-1022` and the drop line on stderr, empty stdout, and that no file is written. The remaining work is one operation that still computes over a dropped model (`quire.properties`), and the missing clean-run AC-23 test for formal-rules coverage, which a pre-existing projection defect blocks.

## Dispositions

Round 1, fix head 7021f18a (rebased onto main with #1040, #1043, #1044 and #1045). Verified against the fix commits a6cfdf64, 32559426 and 7021f18a. Focused tests via locked-build at 7021f18a, all passed: `fr029_dropped_declarations` (10 passed), `quoin-quire-types` (6 passed), `quoin-dossier --lib` (127 passed), `tc_1854_dropped_declarations_a` (12 passed), `tc_1854_dropped_declarations_b` (15 passed), `quoin-suite-trace tc_1854` (2 passed). Mutation re-check: removing the new `properties::classify` check fails `the_properties_classification_refuses_a_dropped_declaration`. Field blast radius: `Obligation.document` is `Option<String>` with `serde(default, skip_serializing_if)`. A scan of every `Obligation { .. }` struct literal under `rust/crates` (tests included) found none without the field. No checked-in JSON schema or golden carries `Obligation`. The change is additive on the wire.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `properties::classify` calls `require_complete_model` after resolve (properties.rs:423-425); `the_properties_classification_refuses_a_dropped_declaration` added; mutant killed. |
| FND-002 | fixed | `quoin_quire_types::Obligation` gains `document: Option<String>`, and `fn obligation` projects `Some(source.document.clone())`; `formal_rules_coverage_without_a_dropped_declaration_reports` exits 0 with a JSON report. |
| FND-003 | fixed | The dossier doc comment now says every dossier command is refused whichever kinds it selects. |
| FND-004 | fixed | The quoin-quire Cargo.toml comment now says the rev is a quire-rs main commit past v0.62.3 whose package still reads 0.62.3, and that it moves to a release tag at the next batched release. |
| FND-005 | fixed | `invoke_refusable`, `invoke_verifying` and `invoke_declining` all use `Response::refusal`; the unused `Diagnostic` import is dropped. |
