---
id: SR-17073
title: "Code review of quoin PR 783 (quoin module inspect, FR-124)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@971b6885; rust/crates/quoin-cli/src/module.rs, rust/crates/quoin-cli/src/tests.rs, rust/crates/quoin-cli/tests/tc_module_command.rs, rust/crates/quoin-core/src/ops/semantic/mod.rs, rust/crates/quoin-core/src/ops/semantic/wire.rs"
review_set: subset
---

# Code review of quoin PR 783

## Summary

Ticket: PLAT-286. Scoped to the one commit unique to the PR (`git log origin/main..HEAD` is
971b6885 alone; the stacked #781 and #782 commits are not on this branch). The Rust review lane
(rust-review, with the repo's `.claude/skills/rust-style`) is folded in here. Tools: quoin
0.29.1, quire 0.36.2 (engine 0.50.2).

Checks run, all from `rust/` into one named target dir: `cargo test -p quoin-cli --test
tc_module_command` passed 6 of 6; `cargo test -p quoin-cli --bins` passed 60 (route table
includes `quoin module inspect` and its `quoin plugin inspect` alias); `cargo test -p quoin-core
--test tc_452_semantic_boundary` passed 5 of 5; `cargo clippy -p quoin-cli --all-targets
--all-features -- -D warnings` and `cargo clippy -p quoin-core --lib --test
tc_452_semantic_boundary -- -D warnings` clean; `cargo fmt --all --check` clean.

The coder's claim holds: `cargo clippy -p quoin-core --all-targets -- -D warnings` fails on
`tests/tc_166_spec_fuzz_skill.rs:79:17` (`clippy::panic`), and that file is byte-identical to
`origin/main`, so the failure is main's and not this PR's.

Stale-lock detection is correct on the path the ticket names: `read_blocks` judges each
validated module's imports against every other validated module in the same request through
the existing `quoin_semantic::resolve_imports` (exact-version match, plus cycle detection), and
`inspect` passes every installed root at once. Reinstalling the imported module at 0.2.0 flags
the importer with "import agent-ix/spec-objects-fixture@0.1.0 is provided by no installed module
(installed: 0.2.0)". Exit and streams follow the boundary contract: a lock or refusal error makes
the outcome `Partial` (exit 1) with the whole document on stdout and one diagnostic per error on
stderr naming the module. `QM020_SEMANTIC_CONTRACT_VIOLATION` is in the catalogue
(`quoin-modules/ERROR_CODES.md`). No digest, pin, fingerprint or count was added, and the FR
explicitly rejects them. The new wire fields are additive and `skip_serializing_if` when empty.

Reproduced FND-001 with the built binary: two modules installed, one module directory removed,
`quoin module inspect` printed `Error: native runtime semantic.read_blocks failed:
CORE_REFUSED: QSEM-001: manifest .../other-mod/manifest.yaml could not be read` and exit 1 with
an empty stdout.

## Verdict

**PASS WITH FINDINGS (one medium, two low).** The feature works and the stale-lock logic is
right. The medium is a real gap in the "whole document" promise: one unreadable installed module
aborts the diagnostic command entirely.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | One installed module whose manifest is missing, unreadable or not YAML makes `semantic.read_blocks` fail the whole request (`?` on `host.read_module`), so `quoin module inspect` writes no document, reports none of the other modules, and exits 1 with prose on stderr. That breaks FR-124's "one entry per installed module" and "still write the complete document". It is also the case an operator runs `inspect` to diagnose. The unreadable module should be an entry with an error diagnostic, not a refusal of the whole report | rust/crates/quoin-core/src/ops/semantic/mod.rs:83-87; rust/crates/quoin-cli/src/module.rs:113-120 |
| FND-002 | low | The lock check clones every other `SemanticModule` once per candidate, so the work is O(n^2) in the number of roots. `read_blocks` bounds the request at 1 MiB, which admits tens of thousands of short or repeated roots, and the per-root bound does not cap that. `resolve_imports` could take borrowed modules, or the population could be built once, which removes the clones | rust/crates/quoin-core/src/ops/semantic/mod.rs:91-103 |
| FND-003 | low | The inspect lock and refusal errors reuse `QM020_SEMANTIC_CONTRACT_VIOLATION`. The catalogue defines that code as "the semantic gate rejects a module; carries the RollbackOutcome". `inspect` is read-only, rejects nothing and carries no rollback. The repo's rule is that a different condition gets its own code, so a caller cannot tell an install rejection from a stale-lock report by code | rust/crates/quoin-cli/src/module.rs:143-145; rust/crates/quoin-modules/ERROR_CODES.md:27 |

## Examined scope

```yaml
clean: false
scope:
- {id: quoin-cli/src/module.rs#inspect, path: rust/crates/quoin-cli/src/module.rs, role: examined, excerpt: "fn inspect() -> Result<Response, String> { let listed = invoke(\"modules.list\", ...)?; ... let read = invoke(\"semantic.read_blocks\", &serde_json::json!({ \"roots\": roots }))?; ..."}
- {id: quoin-cli/src/module.rs#declares_compiled_target, path: rust/crates/quoin-cli/src/module.rs, role: examined, excerpt: "COMPILED_TARGETS: rust, typescript, python-pydantic-v2, python-dataclass"}
- {id: quoin-core/src/ops/semantic/mod.rs#read_blocks, path: rust/crates/quoin-core/src/ops/semantic/mod.rs, role: examined, excerpt: "let lock_diagnostics = result.module.as_ref().map_or_else(Vec::new, |candidate| { ... quoin_semantic::resolve_imports(candidate, &others) });"}
- {id: quoin-core/src/ops/semantic/wire.rs#ModuleSemanticView, path: rust/crates/quoin-core/src/ops/semantic/wire.rs, role: examined, excerpt: "pub name: Option<String>, pub version: Option<String>, ... pub lock_diagnostics: Vec<SemanticDiagnostic>"}
- {id: quoin-cli/src/tests.rs#NATIVE_OWNED_ROUTES, path: rust/crates/quoin-cli/src/tests.rs, role: examined, excerpt: "\"quoin module inspect\", ... \"quoin plugin inspect\","}
- {id: tc_module_command.rs, path: rust/crates/quoin-cli/tests/tc_module_command.rs, role: examined, excerpt: "#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = \"in a test, a panic IS the failure report; the production lints stand\")]"}
- {id: quoin-semantic/src/package_manifest.rs#resolve_imports, path: rust/crates/quoin-semantic/src/package_manifest.rs, role: context_only, excerpt: "pub fn resolve_imports(candidate: &SemanticModule, installed: &[SemanticModule]) -> Vec<SemanticDiagnostic>"}
- {id: tc_166_spec_fuzz_skill.rs, path: rust/crates/quoin-core/tests/tc_166_spec_fuzz_skill.rs, role: context_only, excerpt: "clippy::panic at 79:17; unchanged from origin/main"}
```

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The fix raised the literal count pin `assert_eq!(ModulesErrorCode::all().len(), 23, ...)` from 22 to 23. The pin is ceremony. The next assertion (`bad_request.len() + refused.len() + internal.len() == all().len()`) already fails when a code is added and left out of every group. The per-group `core_code` loops already fail when a grouped code falls through to the `_ => None` arm. Only a new code can change the literal, and the code-reviewed grouping is what decides whether it is handled. Delete the literal assertion, and the doc comment and wildcard-arm comment that cite it, instead of bumping it | rust/crates/quoin-core/src/ops/modules/taxonomy.rs:90-94,110-125 |
| FND-005 | low | `ModulesError::InstalledModuleInvalid { name, detail }` is never constructed anywhere in the workspace. `inspect` uses only `ModulesErrorCode::InstalledModuleInvalid` for its stderr diagnostic. The variant is dead public API with a format string and a `code()` arm. Delete it and keep the code | rust/crates/quoin-modules/src/error.rs:361-372,456 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 1ba15754 and 1ea79a75: `read_blocks` answers a root whose manifest is missing, unreadable, not YAML or not a mapping with `unreadable: <detail>` and keeps going. A missing contract still refuses. `inspect` reports that root as `mode: unreadable` with an error diagnostic and exit 1. tc_286_005 removes an installed module dir and asserts 2 entries, `unreadable` and QM023 on stderr (passes) |
| FND-002 | fixed | `by_package` is built once over the population, and `resolve_imports_in` borrows it. No per-candidate clone remains |
| FND-003 | fixed | New `QM023_INSTALLED_MODULE_INVALID` in `ModulesErrorCode`, ERROR_CODES.md and the taxonomy mapping. `inspect` emits it, and tc_286_005 asserts QM023 is present and QM020 absent |
