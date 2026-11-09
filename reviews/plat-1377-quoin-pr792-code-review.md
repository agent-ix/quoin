---
id: SR-20261103118154
title: "Code review of quoin PR 792 (semantic-core 0.4.3, semantic-schema 0.4.2, retired semantic_core)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3b70d3a1; package.json, pnpm-lock.yaml, rust/crates/quoin-semantic, rust/crates/quoin-core/tests, rust/crates/quoin-cli/tests, rust/crates/quoin-modules/src/contract_gate.rs, rust/crates/quoin-quire/tests/fixtures, templates/semantic-module"
review_set: subset
---

# Code review of quoin PR 792

## Summary

Ticket: PLAT-1377 (also PLAT-278 and PLAT-1221). The review covers `git diff origin/main...HEAD`
at 3b70d3a1 (20 files, +271/-292). The Rust lane (`rust-review`) is folded in. Tools: quoin
0.29.4, quire 0.39.0 (engine 0.55.0). The coder reports that `make test` exited 0 at head, and
it was not re-run here.

Checks run:

- I installed the lockfile with `pnpm install --frozen-lockfile` in a scratch directory. It
  resolved `@agent-ix/semantic-core` 0.4.3 and `@agent-ix/semantic-schema` 0.4.2 from the
  default public registry. The lockfile has no `tarball:` URLs, no `npm.ix` and no
  `npm.pkg.github.com`. The other lockfile changes are transitive patch or minor bumps from
  the regeneration (`@inquirer/*`, `vscode-languageserver*`, `prettier` 3.9.10 and others).
- I computed the `required` arrays of the published 0.4.2 `module-manifest.schema.json` and
  compared them with the `tc_452_641` baseline.
- I compared the strip in `read_semantic_block` with quire-rs `origin/main`
  `src/semantic/contract.rs` and spec FR-069. Both drop `semantic_core` from a clone of the
  block after the contract-version gate and before schema validation, whatever its value. The
  0.4.2 block schema no longer declares `semantic_core` and still has
  `additionalProperties: false`, so the strip is required for such a manifest to load.
- I checked that the strip is load-bearing today. `semantic_core: 0.3.3` is still declared on
  `origin/main` of spec-artifacts-iso and spec-objects-business, and in the local checkouts of
  the other eight default modules.
- I checked that the `tc_278` instances are real fixture data from quire-rs at `ce172a7d`. The
  `type-*` expectations carry only `target`, `decimal` and `unit`, so the instance the test
  builds is the whole expectation.
- Vendoring: no schema was copied into the repo. Schemas still come from `node_modules` and are
  embedded at build time. The `Entity.json` fixture edit only repoints `$ref`s.

## Verdict

**CHANGES REQUESTED.** The pin bump, the FR-071 schema checks, the strip and the fixture cleanup
are correct. The strip matches quire-rs FR-069 exactly, `tc_452_643` stays an exact-set
assertion, and there is no new ceremony. However, the `tc_452_641` guard was widened past what
its constraint allows (FND-001). The cookiecutter hook keeps a vendored copy of the upstream
target registry, and this PR has to hand-sync it (FND-002). FND-003 and FND-004 are low.
FND-004 needs a decision from the owner, not a code change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `tc_452_641` now accepts any new `required` array whose path starts with `/$defs/`. That admits new required arrays nested under the pre-existing `ObjectTypeEntry`, `ArchetypeEntry` and `ArtifactTypeEntry` (for example `/$defs/ArtifactTypeEntry/properties/<p>`), which FR-070-CON-1 forbids. It also silently admits the real break in 0.4.2: `/$defs/LintRuleEntry` gains a root `required: [type, id, section]` plus conditional `column`/`allowed` and `pattern`, and `pattern` changes from an object with required `kind` to a string. The baseline row was deleted, so that change is no longer asserted at all. The new comment ("adds no obligation") is false for a manifest that already sets `lint_rules`. The doc comment still says "14 pairs" and "pre-CR-003 baseline", but the baseline now has 13 pairs and is no longer that set. Fix: either scope the test to FR-070-CON-1 (root, the three entry types and everything under them except `data_schema`, plus the `semantic` block) and correct NFR-017-AC-4, or list the new defs by name as `ConstructDeclaration` was and record the `LintRuleEntry` change as accepted upstream evolution. | rust/crates/quoin-semantic/tests/tc_452_contract_criteria.rs:127-165 |
| FND-002 | medium | The hook's `TARGET_REGISTRY` is a copy of filament-core-data's `common.schema.json` `target` + `representationFormat` enums. Its own comment says "Vendored here" and names a vendored copy under `src/semantic/schemas/filament-core-data/` that PLAT-887 deleted. This PR had to hand-add `sysml-v2-textual` to keep `tc_264_009` green, which is the drift a copy causes. Fix: delete the copy. Rely on the rendered manifest's validation against the published schema (FR-078-AC-1, `tc_264_020`) and on install-time target refusal (FR-070-AC-7). Or have the hook read the registry from the installed package. If FR-076 requires refusal before any file is written, amend it. | templates/semantic-module/hooks/pre_gen_project.py:24-45 |
| FND-003 | low | DIVERGENCE.md still lists `/semantic_core pattern` among the `jsonschema` errors for `semantic-block/many-faults`. With the key stripped and absent from the 0.4.2 schema, that error can no longer occur. | rust/crates/quoin-semantic/DIVERGENCE.md:213 |
| FND-004 | low | Stripping `semantic_core` before validation is a compatibility shim for manifests that carry a retired key. It is load-bearing now, because all ten default modules still declare `semantic_core: 0.3.3`, and quire-rs does the same. But the global rule requires the owner to approve any compatibility layer, and the code records no expiry. PLAT-1221's scope says to delete quire-rs's in-memory patch once the module repos stop declaring the key. Record that same expiry for quoin: delete the strip in both repos when the default modules drop `semantic_core`. | rust/crates/quoin-semantic/src/manifest.rs:342-349 |

## Examined scope

- `read_semantic_block` strip (manifest.rs:342-358). Clean apart from FND-004. Ordering is
  preserved: contract_version is checked first (FR-070-AC-5).
- `SEMANTIC_CONTRACT.semantic_keys` (contract.rs). Nine keys, matching FR-070.
- Helper fixtures in quoin-core, quoin-semantic, quoin-cli and quoin-modules no longer inject
  `semantic_core`. Clean.
- `tc_264_020` (eight keys, raw block validated without a strip). Clean.
- `tc_278_001`, `tc_278_004`, `tc_278_006` schema checks. Clean.
- `tc_452_611b`. Clean as code (gap-analysis FND-001 covers its assertion strength).
- `tc_452_641` (FND-001) and `tc_452_643` (exact six-target set). The latter is clean.
- Template manifest.yaml, conftest.py and hook (FND-002).
- package.json and pnpm-lock.yaml. Clean.
