---
id: PLAN-007
title: "Semantic module contract"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: references
  - target: "ix://agent-ix/quoin/FR-070"
    type: references
  - target: "ix://agent-ix/quoin/FR-071"
    type: references
  - target: "ix://agent-ix/quoin/FR-072"
    type: references
  - target: "ix://agent-ix/quoin/FR-073"
    type: references
  - target: "ix://agent-ix/quoin/FR-074"
    type: references
  - target: "ix://agent-ix/quoin/FR-075"
    type: references
  - target: "ix://agent-ix/quoin/NFR-017"
    type: references
---

# PLAN-007: Semantic module contract

Issue: `agent-ix/quoin#293`. Reviews: SR-118..125 under
`spec/reviews/293-semantic-module-contract/`. Upstream: filament-core-data#34/#35
(merged), filament-core-service#21 (manifest schema, open). Downstream:
quire-rs#388 implements the extraction the fixtures define.

## Requirements Summary

### User Stories

- [ ] **US-020:** A module maintainer declares typed fields, relations, operations, and invariants once against the shared grammar.

### Functional Requirements

- [ ] **FR-070:** Optional `semantic` manifest block with a closed key set; install-time rejections in Quoin; vendored schema with provenance.
- [ ] **FR-071:** Typed Properties table and line-level `sysml` fence map to one normalized `FieldDecl[]`; golden fixtures for quire-rs#388.
- [ ] **FR-072:** Invariants and Operations map to `ClauseRef[]`/`OperationDecl[]`; `Identifier` clause ids; advisory unchecked languages.
- [ ] **FR-073:** `data_schema` by path + digest, offline `$ref` resolution against a vendored semantic-core bundle, inline-schema warning.
- [ ] **FR-074:** Legacy forms at `warning`, `quoin semantic sweep` and the sweep-report guard for promotion.
- [ ] **FR-075:** Derived `package-manifest.json` and per-export digests pinned in `registry.json`; import resolution at `quoin module install`.

### Non-Functional Requirements

- [ ] **NFR-017:** No current manifest or artifact invalidated; advisory by default; no corpus write; no required key.

## Dependency Graph

```text
TASK-039 -> TASK-040 -> TASK-042 --\
        \-> TASK-041 --------------+-> TASK-043 -> TASK-044
```

- TASK-039 lands the schema change in filament-core-service#21 and vendors both schemas (module manifest, semantic-core bundle) with provenance; every later task validates against them.
- TASK-040 (manifest block, `data_schema` references, authoring pack) and TASK-041 (mapping fixtures, legacy forms, sweep) touch disjoint code and run in parallel.
- TASK-042 (package manifest derivation, registry pins) needs the loaded block and the schema digests from TASK-040.
- TASK-043 runs the NFR-017 gates over the finished slice; TASK-044 is the review gate.

### Cross-cutting constraints

- Quire-executed behaviour (extraction, artifact-time diagnostics) is specified here by fixtures and executed by quire-rs#388; quoin tests verify the fixtures' expected outputs against the vendored schemas and each other, not extraction itself.
- `quoin` performs no network read on any command path (FR-073-CON-1) and writes nothing outside `IX_HOME` and the module root.
- quoin's two pre-existing uncommitted ignore-file edits are not part of this plan; stage `spec/`, `src/`, `tests/`, `scripts/`, `plan/` paths explicitly.

## The Seams

`src/catalog.ts` `loadCatalog` gains the `semantic` block reader and install-time
validation (via `src/commands/module/install.ts`); `src/write.ts` reports the
block in the authoring pack; a new `src/semantic/` module holds the schema
vendoring, `data_schema` reference resolution, package-manifest derivation,
registry pins, and the sweep command; `scripts/refresh-manifest-schema.mjs` and
`scripts/refresh-semantic-core-schemas.mjs` follow `refresh-quire-schemas.mjs`.
Fixtures live under `tests/fixtures/semantic-module/`.

## Test Plan

- [ ] **TC-1342, TC-1343, TC-1385:** schema diffs and vendored provenance.
- [ ] **TC-1336..1341, TC-1383, TC-1337:** manifest block load, rejections, pack.
- [ ] **TC-1360..1366:** `data_schema` references, failures, offline resolution.
- [ ] **TC-1344..1352, TC-1384:** Properties mapping fixtures and cell grammars.
- [ ] **TC-1353..1359:** Invariants/Operations fixtures.
- [ ] **TC-1367..1371, TC-1386:** legacy forms, promotion guard, sweep.
- [ ] **TC-1372..1378:** derived package manifest and registry pins.
- [ ] **TC-1379..1382:** NFR-017 gates.

### Entrance Criteria

- US-020, FR-070..075, NFR-017, TC-1336..1386, SR-118..125 validate with Quire (done 2026-09-03).
- filament-core-data#35 merged (semantic-core 0.1.0 `generated/json-schema` + `toolchain.json` available to vendor).

### Exit Criteria

- All quoin-executable TC-1336..1386 rows pass; quire-only rows have fixture-level evidence and a named #388 hand-off.
- Every default module loads unchanged; schema `required` arrays unchanged; no corpus path in the diff.
- Code review and gap analysis report no blocking finding; PR carries a "mergeable" comment.

## Remaining Work

### Track A: Critical Path

- **A1 = TASK-039** Schema ownership and vendoring — Medium.
- **A2 = TASK-040** Manifest block, `data_schema` references, authoring pack — Hard.
- **A3 = TASK-042** Package manifest derivation and registry pins — Medium.

### Track B: Parallel after vendoring

- **B1 = TASK-041** Mapping fixtures, legacy forms, sweep — Hard.

### Track C: Gates

- **C1 = TASK-043** NFR-017 gates — Small.
- **Gate = TASK-044** Review, gap analysis, PR.

## Task File Mapping

| Task     | Track | Owns (references)      | Verified by (verifies)                         | Status |
| -------- | ----- | ---------------------- | ---------------------------------------------- | ------ |
| TASK-039 | A     | FR-070, FR-073         | TC-1342, TC-1343, TC-1385                      | done   |
| TASK-040 | A     | FR-070, FR-073         | TC-1336..1341, TC-1360..1366, TC-1383          | todo   |
| TASK-041 | B     | FR-071, FR-072, FR-074 | TC-1344..1359, TC-1367..1371, TC-1384, TC-1386 | todo   |
| TASK-042 | A     | FR-075                 | TC-1372..1378                                  | todo   |
| TASK-043 | C     | NFR-017                | TC-1379..1382                                  | todo   |
| TASK-044 | Gate  | US-020                 | —                                              | todo   |

## Coordination Rules

- Only TASK-039 touches vendored schema files; later tasks read them.
- `src/semantic/` is single-writer per task: install-time validation (040), fixtures/sweep (041), derivation/pins (042).
- Merge sequencing: TASK-039..043 on `spec/293-semantic-module-contract`, then `/code-review` and `/gap-analysis`, then the "mergeable" comment and squash merge verified on the tree.
