---
id: SR-126
title: "Code review of the semantic module contract"
type: SpecReview
analysis: code-review
scope: "src/semantic/, src/commands/semantic/, src/plugins.ts, src/catalog.ts, src/write.ts, scripts/refresh-*.mjs, tests/semantic-*.test.ts, tests/fixtures/semantic-module/, vite.config.ts"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-007"
    type: reviews
---

# Code review of the semantic module contract

## Summary

Issue #293 adds the `semantic` manifest block with install-time rejection and
rollback, `data_schema` references resolved offline against a vendored
semantic-core bundle, golden mapping fixtures for quire-rs#388, the legacy-form
sweep and its promotion guard, and derived package manifests with registry
pins. Two defects were repaired during review; the remaining findings are
accepted limits with their consequence named.

## Verdict

**CONDITIONAL** — no open high finding; the two highs found were fixed before
this artifact was written.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                                                                                                                                             | Refs                                                                           |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| FND-230 | high     | Resolved during review: a rejected re-install of an already-installed module removed the previously good version, because `installEntry` overwrites the materialized copy before the block can be read; `installPlugin` now snapshots the registry and reinstalls the previous entry from its recorded source on rejection (covered by a new test). | src/plugins.ts, tests/semantic-manifest.test.ts                                |
| FND-231 | medium   | Resolved during review: the sweep classifier's table-start rule carried a spurious `previous.line + blocks.length < i` condition that could miss a second table after a list; a table now starts only on a header row followed by a separator row.                                                                                                  | src/semantic/sweep.ts                                                          |
| FND-232 | low      | Accepted: `loadCatalog` reads every module's semantic block on each load, which hashes each referenced schema file; the Ajv compile is cached, the hashing is per object type and bounded by module size. Revisit if catalog load time becomes visible.                                                                                             | src/catalog.ts                                                                 |
| FND-233 | low      | Accepted: the FR-075 pin lives as an extra key on the ts-plugin-kit registry entry; a later ts-plugin-kit upsert of that entry drops it until the next `quoin module install` recomputes it. quoin#287 may relocate the pins.                                                                                                                       | src/plugins.ts                                                                 |
| FND-234 | low      | The refresh scripts read provenance out of `src/semantic/contract.ts` with regular expressions, the same pattern `refresh-quire-schemas.mjs` uses; a reshaped constant fails loudly rather than silently.                                                                                                                                           | scripts/refresh-manifest-schema.mjs, scripts/refresh-semantic-core-schemas.mjs |
| FND-235 | low      | Pre-existing, not touched: `tests/quire-contract.test.ts` TC-118 fails on this host because the installed `quire` 0.31.0 is ahead of `quality/verification-stack-lock.json` and emits a `criteria` field outside the vendored v1 coverage schema; `make test` stops on the same lock mismatch.                                                      | tests/quire-contract.test.ts, quality/verification-stack-lock.json             |
| FND-236 | low      | No stub, skipped case, mock, placeholder return, `TODO`, warning suppression, or unowned production behavior was found; `package.json` and `pnpm-lock.yaml` are untouched; vendored bytes are excluded from prettier and hash-asserted.                                                                                                             | PLAN-007                                                                       |

## Test and Boundary Review

- Six new Vitest files, 48 cases, tag every quoin-executed TC-1336..1386 row;
  quire-executed rows have fixture-level evidence recorded in
  `tests/fixtures/semantic-module/mapping/README.md`.
- Boundaries exercised: unknown key, undeclared export, unsupported contract
  version (gated first), unknown target, malformed package, duplicate package
  across modules (sorted root order), digest mismatch, missing/non-JSON/`$id`-
  less schema, `..` and symlink escapes, ambiguous mixed `data_schema`, `$ref`
  to another semantic-core version, unshipped `$ref`, `$ref` cycle, inline
  schema warning only under a block, sweep-report guard (missing, other
  package, other version), unresolved import with installed versions named,
  import cycle, URL/`ix://` package, rejected re-install rollback.
- Offline: the resolution path has no network import; the derived manifest is
  validated against the vendored filament-core-data schema before it is
  written.

## Spec-Code Faithfulness

- FR-070: `readSemanticBlock` + install-time rejection in Quoin, authoring
  pack lines, vendored schema with provenance.
- FR-071/FR-072: fixtures and expectations; quoin verifies them against the
  vendored semantic-core 0.1.0 schemas; execution is quire-rs#388.
- FR-073: reference form, raw-byte digest, `$id` scheme, offline `$ref`
  resolution, inline warning.
- FR-074: classifier, `quoin semantic sweep`, report schema, promotion guard,
  pack migration example.
- FR-075: derivation aligned with the vendored v1 package-manifest schema
  (FR-075 text amended for `schemaDialect` and import/export/profile shapes),
  registry pins, import resolution.
- NFR-017: default modules load unchanged, warning-only sweep, change-set and
  `required`/lockfile gates.
