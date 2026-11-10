---
id: SR-20261103118169
title: "Code review of quoin PR 797 (PLAT-212 import_types, unknown-import-type, unused-import)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@f1e6b687; rust/crates/quoin-semantic/src/{data_schema.rs,manifest.rs,manifest/import_types.rs,package_manifest.rs,contract.rs,diagnostic.rs}, rust/crates/quoin-semantic/tests/tc_212_import_types.rs, rust/crates/quoin-cli/tests/tc_module_command.rs, package.json, pnpm-lock.yaml, spec/matrix.md"
review_set: subset
---

# Code review of quoin PR 797

## Summary

Ticket: PLAT-212. Scoped to `git diff origin/main...HEAD` at f1e6b687 (14 files, +974/-50).
Rust lane (`rust-review`, with the repo's `.claude/skills/rust-style`) folded in. Tool:
quoin 0.29.4. Gates were not re-run on instruction (the coder reported `make test` green on
this head); this review reads the code.

Checks made:

- `$ref` walker: refs under a declared import's base (`https://schemas.agent-ix.org/<org>/<repo>/`,
  trailing slash, so `org/repo` cannot prefix `org/repo-x`) are skipped; refs under the own
  base still need a shipped file inside the module root; everything else (including an
  undeclared package) still falls to `semantic.schema-ref-unshipped`. The own-base branch is
  tested before the import branch and two two-segment identities cannot nest, so an import
  cannot shadow the own base. One escape remains (FND-001).
- `semantic.unknown-import-type`: list form at `semantic.import_types.<pkg>`, `$ref` form at
  `<section>[<type>].data_schema.schema`, both only when the provider is installed at the
  declared version (`filter(|m| m.version == version)`); deduped per (locus, package, type).
- `semantic.unused-import` is a warning; `install.rs` and `contract_gate.rs` fail only on
  `has_errors`, so install still succeeds; `inspect` exit-zero is covered by tc_212_002 (CLI).
- Shape rules (`manifest/import_types.rs`) run before the schema and return
  `semantic.invalid-value` naming package and entry for all six AC-9 shapes plus non-object.
- Locus fix: `declared_types` now carries its section, and `ResolveContext::locus` and
  `ResolvedDataSchema.locus` use it, so `artifact_types` entries no longer report under
  `object_types[...]`.
- Derived `exports` = declared `import_types` list or `[]`.
- Cycle naming: every package cycle still produces `semantic.import-cycle`; `type_cycle`
  only picks the message. Iterative DFS with a `done` set is correct and terminates.
- `package.json` / `pnpm-lock.yaml`: `@agent-ix/semantic-schema` 0.4.2 -> 0.5.0; the lock
  integrity `sha512-7iGPuc1B...` matches the npmjs 0.5.0 `dist.integrity` in the local pnpm
  metadata cache. `contract.rs` admits `import_types`; tc_452_612 asserts the schema accepts it.
- `SemanticBlock.import_types` has `skip_serializing_if = "BTreeMap::is_empty"`, so
  existing serialized blocks are unchanged (asserted in tc_212_001). `SemanticBlock` is
  `Serialize`-only, so no deserialization default is needed.
- Tests carry `#[trace("AC", "TC")]` markers with the matrix TC ids; SPDX headers and
  `//!` module docs present; no `unwrap`/indexing in production code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A `$ref` under a declared import's base containing `..` segments (e.g. `<import-base>../other-repo/T.json`) is now silently accepted, escaping to an undeclared package; it was refused as schema-ref-unshipped before | rust/crates/quoin-semantic/src/data_schema.rs:521-527 |
| FND-002 | low | A data_schema that fails resolution returns empty `file_refs`, so an import it would have referenced also gets a spurious `semantic.unused-import` warning beside the real error | rust/crates/quoin-semantic/src/data_schema.rs:295-297; rust/crates/quoin-semantic/src/package_manifest.rs:412 |
| FND-003 | low | `type_cycle` searches every type edge among the cycle's packages, so a longer package cycle (A->B->C->A) can be labelled with a shorter type cycle among a subset (A#X -> B#T -> A#X), giving two identical messages for two different cycles | rust/crates/quoin-semantic/src/package_manifest.rs:571-573 |
| FND-004 | low | Test comment says "The provider moves on to export nothing the importer uses" but the test edits the consumer's own `import_types` and `$ref`, not the provider | rust/crates/quoin-cli/tests/tc_module_command.rs:318 |

## Verdict

**PASS WITH FINDINGS (one medium, three low).** The settled design is implemented as stated:
sibling optional `import_types`, shape errors as `semantic.invalid-value`, unknown-type errors
at both loci gated on the provider being installed at the declared version, unused-import as
a non-failing warning, derived exports, and cycle naming that never suppresses a package-cycle
failure. The walker change does not weaken refusal of refs to undeclared packages except
through `..` path segments (FND-001), which should be refused (reuse `has_parent_segment` on
the remainder after the base) before merge. FND-002..004 are low and may ride the same fix
round.

## Scope

- `rust/crates/quoin-semantic/src/data_schema.rs` (examined): RefWalker import-base skip, FileRefs, schema_base, ref_type_name, section-aware locus.
- `rust/crates/quoin-semantic/src/manifest.rs` (examined): import_types field, shape pre-check, (section, entry) declared_types, ResolveContext imports.
- `rust/crates/quoin-semantic/src/manifest/import_types.rs` (examined): shape rules, no findings.
- `rust/crates/quoin-semantic/src/package_manifest.rs` (examined): derived exports, import_type_diagnostics, type_graph, type_cycle, CycleVisitor naming.
- `rust/crates/quoin-semantic/src/contract.rs`, `diagnostic.rs` (examined): new key and codes, no findings.
- `rust/crates/quoin-semantic/tests/tc_212_import_types.rs` (examined): tc_212_001..008.
- `rust/crates/quoin-cli/tests/tc_module_command.rs` (examined): tc_212_001..002 (CLI inspect).
- `package.json`, `pnpm-lock.yaml` (examined): 0.5.0 bump and integrity, no findings.
- `rust/crates/quoin-modules/src/contract_gate.rs`, `install.rs` (context_only): install fails on errors only.

## Dispositions

Round 1, fix head ed89c79d, verified against `git diff f1e6b687 ed89c79d`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | data_schema.rs:521-524 now skips a ref only when `url.strip_prefix(imported).is_some_and(\|rest\| !has_parent_segment(rest))`; a `..` remainder falls to the schema-ref-unshipped refusal. Asserted by tc_212_010 (`dotdot` case). |
| FND-002 | fixed | package_manifest.rs:412-418 suppresses the unused-import warning when any data schema has `schema: None` (failed to resolve), so the real error stands alone. |
| FND-003 | fixed | `type_cycle` now takes the closed package path and keeps only type edges whose package pair is consecutive on it; tc_212_009 asserts the three-package cycle is named along its own edges (it would have been named F#E -> O#E -> F#E before). |
| FND-004 | fixed | Comment at tc_module_command.rs:318 now reads "The importer moves on to list and `$ref` types the provider does not export." |
