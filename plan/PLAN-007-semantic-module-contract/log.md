---
type: log
title: "PLAN-007 — Update Log"
description: "Chronological log of changes to the PLAN-007 bundle."
---

# PLAN-007 — Update Log

## History

- **2026-09-03** — Plan created from the validated issue #293 specification (US-020, FR-070..075, NFR-017, TC-1336..1386, SR-118..125) with six tasks: schema ownership and vendoring, manifest block and `data_schema` references, mapping fixtures and sweep, package manifest derivation, NFR-017 gates, review gate.
- **2026-09-03** — TASK-039 done: filament-core-service#22 merged (a77f31e; FR-035 CR-003; the two shipped schema copies had drifted and were reconciled). Vendored into `src/semantic/schemas/`: module-manifest schema, semantic-core 0.1.0 bundle (30 files + toolchain.json, digest equal to filament-core-data `toolchain.json` at d48b8da), package-manifest and common schemas; provenance in `src/semantic/contract.ts`; refresh scripts refuse drift; vendored dirs added to `.prettierignore` (prettier had reformatted the JSON and broken the hashes — the guard worked). TC-1342, TC-1343, TC-1385 (+ TC-1382) green.
- **2026-09-03** — TASK-040 done: `src/semantic/manifest.ts` (block read against the vendored schema; contract-version gate first; unknown key, undeclared export, unknown target, malformed package, unknown semantic-core, sweep-report guard), `src/semantic/data-schema.ts` (reference form, raw-byte digest, escape/symlink/ambiguous/missing/non-JSON/`$id` checks, `$ref` resolution against the shipped bundle and vendored semantic-core with version and cycle diagnostics, inline-schema warning), install-time rejection with rollback and duplicate-package detection in `src/plugins.ts`, block carried on `SpecModule` and reported by the authoring pack. Fixture module `tests/fixtures/semantic-module/module-ok`. TC-1336..1341, TC-1360..1366, TC-1379, TC-1383 green (19 cases).
- **2026-09-03** — TASK-041 done: golden mapping fixtures under `tests/fixtures/semantic-module/mapping/` (FR-006 typed table and `sysml` fence sharing one expected `FieldDecl[]`, both-forms, 24 cell/fence-line/reader-rule cases, Invariants/Operations fixture and 10 diagnostic cases, legacy bullet/mixed cases, pinned unmodified FR-006 copy with provenance, README hand-off to quire-rs#388); `src/semantic/sweep.ts` classifier + `quoin semantic sweep` + `sweep-report.schema.json`; promotion guard exercised with real reports; authoring pack shows the migration once. Quoin proves every expectation validates against the vendored semantic-core 0.1.0 schemas; extraction execution is #388. TC-1344..1359, TC-1367..1371, TC-1380, TC-1384, TC-1386 green.
