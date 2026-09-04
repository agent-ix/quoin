---
id: FR-076
title: "Semantic-module template variants from one maintained core"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
---

# FR-076: Semantic-module template variants from one maintained core

## Description

Quoin SHALL ship one maintained cookiecutter template that renders a Quire
semantic-module repository in an artifact, object, or mixed variant from a single
template core, so that a variant is a rendering decision rather than a copied
template.

## Rationale

Three copied cookiecutters would reproduce, one level up, exactly the drift this
requirement exists to end: the shared surfaces would be maintained three times
and would diverge on whichever one a maintainer edited. One core with a
`module_kind` switch keeps the shared surfaces single-sourced and confines the
difference to the manifest section a kind contributes, the skeletons it ships,
and the mapping declarations it needs.

## Inputs

- `org`, `repo_name`, `description`, `module_name`, `package_name`, `author`, `email`, `version`
- `module_kind`, one of `artifact`, `object`, `mixed`
- `semantic_core_version`, the exact `@agent-ix/semantic-core` version to import
- `generated_targets`, a subset of the filament-core-data target registry
- `imported_modules`, zero or more `<org>/<repo>@<exact-version>` references
- `license`, defaulting to `AGPL-3.0-or-later`

## Outputs

- A rendered repository directory named by `repo_name`
- A rendering failure naming the offending input, when an input is rejected

## Behavior

- The template SHALL render an `artifact`, an `object`, or a `mixed` module repository from one template core, selected by `module_kind`.
- Where a surface is shared by every variant, the template SHALL carry exactly one copy of that surface.
- When `module_kind` is `object`, the rendered manifest SHALL declare its exported types under `object_types` and no `artifact_types` entry.
- When `module_kind` is `artifact`, the rendered manifest SHALL declare its exported types under `artifact_types` and no `object_types` entry.
- When `module_kind` is `mixed`, the rendered manifest SHALL declare both an `artifact_types` and an `object_types` section.
- When `module_kind` is `mixed`, the rendered `semantic.imports` SHALL carry at least one imported package identity.
- The template SHALL default `license` to `AGPL-3.0-or-later`.
- The template SHALL render the full licence text of the `license` value it accepts.
- If `license` names anything other than an AGPL-3.0-or-later identifier, then the template SHALL abort rendering naming the value, rather than rendering a repository whose licence declarations disagree.
- If `module_kind` names a value outside `artifact`, `object`, and `mixed`, then the template SHALL abort rendering naming the value.
- If an entry of `imported_modules` is not of the `<org>/<repo>@<exact-version>` form, then the template SHALL abort rendering naming the entry.
- If an entry of `generated_targets` is outside the filament-core-data target registry, then the template SHALL abort rendering naming the entry.
- The template SHALL record every target it declares but does not emit today in the rendered README and Test Matrix, rather than presenting a declared target as an emitted one.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-076-CON-1 | The template SHALL contain no copy of the schema emitter, the Quire runtime, or the semantic-core grammar; each is reached as a versioned dependency. | Architecture | Inspection |
| FR-076-CON-2 | The rendered repository SHALL contain no `.npmrc`; `@agent-ix` resolves from the user-level npm configuration. | Packaging | Test (TC-1412) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-076-AC-1 | Rendering with `module_kind: object` produces a repository whose manifest declares `object_types` and no `artifact_types`. | Test (TC-1400) |
| FR-076-AC-2 | Rendering with `module_kind: artifact` produces a repository whose manifest declares `artifact_types` and no `object_types`. | Test (TC-1401) |
| FR-076-AC-3 | Rendering with `module_kind: mixed` produces a repository declaring both sections and a non-empty `semantic.imports`. | Test (TC-1402) |
| FR-076-AC-4 | Every file byte-identical across the three rendered variants exists exactly once in the template source. | Test (TC-1403) |
| FR-076-AC-5 | `license: MIT` aborts rendering naming `MIT`; the default renders the AGPL-3.0-or-later text. | Test (TC-1404) |
| FR-076-AC-6 | `module_kind: hybrid` aborts rendering naming `hybrid`. | Test (TC-1405) |
| FR-076-AC-7 | `imported_modules: ["agent-ix/spec-objects-business"]` aborts naming the entry; `agent-ix/spec-objects-business@0.3.0` renders. | Test (TC-1406) |
| FR-076-AC-8 | `generated_targets: ["go"]` aborts naming `go`. | Test (TC-1407) |
| FR-076-AC-9 | No rendered variant contains an `.npmrc` file at any depth. | Test (TC-1412) |

## Dependencies

- **Upstream**: [US-021](../usecase/US-021-generate-a-conforming-semantic-module-repository.md), [FR-070](./FR-070-semantic-module-manifest-extension.md)
- **Downstream**: [FR-077](./FR-077-generated-schema-emission.md), [FR-078](./FR-078-generated-manifest-semantic-block.md), [FR-083](./FR-083-template-render-self-tests.md)
