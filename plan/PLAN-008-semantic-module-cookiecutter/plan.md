---
id: PLAN-008
title: "Semantic module cookiecutter"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/quoin/StR-008"
    type: references
  - target: "ix://agent-ix/quoin/US-021"
    type: references
  - target: "ix://agent-ix/quoin/FR-076"
    type: references
  - target: "ix://agent-ix/quoin/FR-077"
    type: references
  - target: "ix://agent-ix/quoin/FR-078"
    type: references
  - target: "ix://agent-ix/quoin/FR-079"
    type: references
  - target: "ix://agent-ix/quoin/FR-080"
    type: references
  - target: "ix://agent-ix/quoin/FR-081"
    type: references
  - target: "ix://agent-ix/quoin/FR-082"
    type: references
  - target: "ix://agent-ix/quoin/FR-083"
    type: references
  - target: "ix://agent-ix/quoin/NFR-018"
    type: references
  - target: "ix://agent-ix/quoin/NFR-019"
    type: references
  - target: "ix://agent-ix/quoin/NFR-020"
    type: references
---

# PLAN-008: Semantic module cookiecutter

Issue: `agent-ix/quoin#307`, under EPIC `agent-ix/quoin#286`. Reviews: SR-128..135
under `spec/reviews/module-cookiecutter-*.md`. Upstream: the semantic-module
contract (`agent-ix/quoin#293`, merged `3e842ce`), filament-core-data ADR-0005,
`@agent-ix/semantic-core` 0.1.0. Ground truth: the two completed hand migrations,
`spec-objects-business#4` (`567e5c4`) and `spec-artifacts-iso#34` (`6686f11`).

## Requirements Summary

### Stakeholder and User Stories

- [ ] **StR-008:** A new semantic-module repository conforms by construction, not by review after the fact.
- [ ] **US-021:** A maintainer generates the repository from one maintained template and fills in only their module's vocabulary.

### Functional Requirements

- [ ] **FR-076:** Artifact, object and mixed variants from one template core; AGPL by default with no silent fallback; every invalid input aborts naming the value; unattended rendering.
- [ ] **FR-077:** TypeSpec source importing `@agent-ix/semantic-core`; deterministic emission through the official emitter; absolute `$id`/`$ref`; check mode.
- [ ] **FR-078:** The `semantic` manifest block with the nine admitted keys and reference-form `data_schema`; machine-written digests; unique export names.
- [ ] **FR-079:** Typed-table skeletons with `sysml` alternates and `ocl` invariants; one negative fixture per declared failure mode; a legacy-form fixture at `warning`.
- [ ] **FR-080:** The rendered suite treats the engine as a hard dependency and fails, naming `make dev-quire`, rather than skipping.
- [ ] **FR-081:** Public AGPL baseline: one licence identifier, the ownership and guidance files, matching Python and npm payloads, no credential, no private publication default.
- [ ] **FR-082:** The rendered `spec/` tree validates as rendered, with a Test Matrix drawn from the archetype's vocabulary and honest `🚧` rows.
- [ ] **FR-083:** Quoin's gate renders every variant into a temporary directory, checks it against a declared conformance contract, and fails on drift from the maintained repositories at pinned revisions.

### Non-Functional Requirements

- [ ] **NFR-018:** No generation residue in any rendered file; the residue classes are declared patterns, not judgement calls.
- [ ] **NFR-019:** Rendering twice and emitting twice are byte-identical.
- [ ] **NFR-020:** Every external command has a declared floor and a named absent-tool diagnostic.

## Dependency Graph

```text
TASK-045 -> TASK-046 -> TASK-047 -> TASK-048 -> TASK-050 -> TASK-051
        \-> TASK-049 -----------------------/
```

- TASK-045 is enablement for everything else: the template skeleton, the input contract, the hooks, and the conformance contract. Nothing can render until it exists.
- TASK-046 (schema source and emit pipeline) and TASK-049 (public packaging baseline and governance tree) touch disjoint rendered surfaces and run in parallel after TASK-045.
- TASK-047 (manifest `semantic` block and digests) needs the emitted schemas TASK-046 produces.
- TASK-048 (skeletons, mappings, fixtures, and the rendered suite) needs the manifest TASK-047 writes.
- TASK-050 is the gate in this repository: it renders all three variants, scans for residue, checks conformance and drift, and is the first thing that proves the template works rather than reads well.
- TASK-051 closes the cycle — code review, gap analysis, the pull request.

## Execution Tracks

| Track | Tasks | Runs |
| --- | --- | --- |
| A — template core | TASK-045, TASK-046, TASK-047, TASK-048 | serial |
| B — repository baseline | TASK-049 | parallel with A after TASK-045 |
| C — verification | TASK-050, TASK-051 | after A and B |

## Quality Gates

| Gate | Condition | Blocks |
| --- | --- | --- |
| G1 — renders | All three variants render unattended with no error. | TASK-046 onward |
| G2 — emits | A rendered variant emits one schema per exported type, twice, byte-identically. | TASK-047 |
| G3 — declares | The rendered manifest validates against the vendored module-manifest schema and every digest matches. | TASK-048 |
| G4 — verifies | The rendered suite runs, fails without the engine, and reports zero skips with it. | TASK-050 |
| G5 — conforms | Quoin's gate passes: conformance, residue, determinism, drift, and `quire validate` over each rendered spec tree. | TASK-051 |
| G6 — repo green | `make lint` and `make test` pass in this repository, and `quire validate --scope . "spec/**/*.md"` is structurally clean. | the pull request |

## Test Plan

Every test lands in this repository, because the rendered repositories are
disposable: a template's tests are tests of what it renders, and keeping them
here is what makes the template verified rather than merely written.

| Test group | File | Test cases |
| --- | --- | --- |
| Input contract and variants | `tests/semantic-module-template.test.ts` | TC-1400..TC-1407, TC-1412, TC-1451..TC-1456, TC-1466, TC-1467 |
| Emission | `tests/semantic-module-template.test.ts` | TC-1408, TC-1409, TC-1413..TC-1416, TC-1449, TC-1457 |
| Manifest and digests | `tests/semantic-module-template.test.ts` | TC-1410, TC-1417..TC-1419, TC-1458, TC-1459, TC-1468 |
| Skeletons and fixtures | `tests/semantic-module-template.test.ts` | TC-1411, TC-1420..TC-1426 |
| Rendered suite behaviour | `tests/semantic-module-template.test.ts` | TC-1427..TC-1431, TC-1450, TC-1460, TC-1469 |
| Packaging baseline | `tests/semantic-module-template.test.ts` | TC-1432..TC-1438 |
| Governance tree | `tests/semantic-module-template.test.ts` | TC-1439..TC-1443, TC-1461, TC-1470 |
| Gate, conformance and drift | `tests/semantic-module-template.test.ts` | TC-1418, TC-1444..TC-1448, TC-1462..TC-1465 |

## Out of Scope

- Adopting the template in an existing module repository. The issue's safety gate
  makes that separate reviewable work under each module's own schema-completion
  ticket, and this plan writes to no module repository.
- Emitters for the `rust`, `typescript`, `python-pydantic-v2` and
  `python-dataclass` targets. Those belong to `agent-ix/filament-core-data#11`;
  this plan declares the targets and records them as not emitted.
- Publishing anything. No tag, no registry, no catalog entry is created here.
