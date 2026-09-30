---
id: FR-082
title: "Generated governance tree validates as rendered"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-015"
    type: "depends_on"
---

# FR-082: Generated governance tree validates as rendered

## Description

The rendered repository SHALL carry a `spec/` tree that passes `quire validate`
structurally as rendered, and whose acceptance criteria are bound to the
rendered tests by trace tag rather than by a hand-written Test Matrix (CR-001).

## Rationale

A rendered spec tree that fails validation makes the first act in a new
repository a repair, and teaches the maintainer that the gate is noise. A
hand-written Test Matrix was the sharper case: its `Status` cells drifted from
what the tests showed, and `⚠️`, admitted by an older contract and classed as
nothing, exempted a row from the status-lie check by construction. The matrix is
now computed by `quire matrix` from the criteria and the tests' trace tags, so
the rendered repository ships the two inputs and no copy of the output.

## Inputs

- The rendered repository identity and exported types

## Outputs

- `spec/spec.md`, `spec/index.md`, `spec/log.md`
- `spec/stakeholder/`, `spec/usecase/`, `spec/functional/`, `spec/non-functional/`, each with an index

## Behavior

- The rendered `spec/` tree SHALL validate structurally under `quire validate` with no error, as rendered and before any editing.
- The rendered `spec/` tree SHALL carry a master-requirements root, a stakeholder requirement, a user story, and the functional requirements that describe the obligations the rendered REPOSITORY carries — its manifest block, its emitted schemas, its skeletons, its packaging, and its gate.
- The rendered `spec/` tree SHALL state, in its master-requirements Out of Scope section, that the module's own domain types are specified by its maintainer and are not supplied by the template.
- The rendered `spec/` tree SHALL carry an index and a log for each folder that the reserved archetypes require one for.
- The rendered `spec/` tree SHALL NOT carry a hand-written Test Matrix (`spec/matrix.md` or `spec/tests.md`).
- Every rendered acceptance criterion's `Verification` cell SHALL name a verification method only, with no test-case id.
- Every rendered test SHALL carry a trace tag naming an acceptance criterion of the rendered spec, by that criterion's own id.
- The rendered repository's gate SHALL run `quire validate` over the rendered `spec/` tree, so a later edit that breaks it fails the gate.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-082-CON-2 | The rendered spec tree SHALL describe the rendered module, carrying no requirement copied from an existing module repository. | Independence | Test (TC-1461) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-082-AC-1 | `quire validate` over each rendered variant's `spec/**/*.md` exits zero with no error diagnostic. | Test (TC-1439) |
| FR-082-AC-2 | No rendered variant carries `spec/matrix.md` or `spec/tests.md`, and the rendered `spec/index.md` links neither. | Test |
| FR-082-AC-3 | No rendered acceptance criterion's `Verification` cell carries a test-case id. | Test |
| FR-082-AC-4 | Every rendered test function carries a trace tag, and every id it names is an acceptance criterion that exists in the rendered spec. | Test |
| FR-082-AC-5 | Each rendered variant carries the master-requirements root, the stakeholder, usecase, functional, and non-functional folders, and their indexes. | Test (TC-1443) |
| FR-082-AC-6 | The rendered master-requirements Out of Scope section states that the module's domain types are the maintainer's to specify, and no rendered requirement text is copied from a maintained module repository. | Test (TC-1461) |

## Dependencies

- **Upstream**: [FR-015](./FR-015-emit-quire-validate-command.md), [FR-076](./FR-076-semantic-module-template-variants.md)
- **Downstream**: [FR-083](./FR-083-template-render-self-tests.md)

> **CR-001 (2026-09-29, PLAT-1083):** the rendered repository no longer ships a
> hand-written Test Matrix. `quire matrix` computes it from the rendered
> criteria and the rendered tests' `@pytest.mark.trace` tags, and `quoin matrix`
> adds run evidence. AC-2..AC-4 are restated from the matrix's `Status` cells and
> row traces to the absence of a matrix file, method-only `Verification` cells,
> and criterion-id trace tags on every rendered test. FR-082-CON-1, the rendered
> matrix's `Status` vocabulary, is withdrawn: there is no rendered `Status` cell
> left for it to constrain. The `TestMatrix` archetype itself stays valid for
> repositories that still carry one.
>
> The rendered NFR-001-AC-1 (zero skipped tests) and NFR-001-AC-2 (an absent
> tool fails naming its install command) were recorded in the deleted matrix as
> `🚧` rows, and no rendered test asserts either. AC-1 is now verified `Manual`
> (the skip count read from a run's summary) and AC-2 `Inspection` (of the
> absent-tool paths in `tests/conftest.py`, `tests/test_schema_emission.py` and
> `scripts/generate-schemas.mjs`). Both methods compute as
> `method-without-symbol`, so a fresh render carries no untagged criterion.
