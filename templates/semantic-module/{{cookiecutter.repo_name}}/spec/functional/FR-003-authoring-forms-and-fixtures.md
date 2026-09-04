---
id: FR-003
title: "Demonstrate the authoring forms and exercise their refusals"
type: FR
relationships:
  - target: "ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001"
    type: "implements"
---

# FR-003: Demonstrate the authoring forms and exercise their refusals

## Description

The module SHALL ship, for every exported type, an authoring skeleton in the
typed-table form, a `sysml`-fence alternate declaring the same fields, and one
fixture per failure mode its emitted schemas refuse.

## Outputs

- `{{ cookiecutter.package_name }}/skeletons/<type>.md` and `<type>.sysml.md`
- `tests/fixtures/negative/*.md` and `tests/fixtures/legacy/*.md`

## Behavior

- Each skeleton SHALL carry a `## Properties` section whose table header is exactly the typed four-column form.
- Each skeleton SHALL be accompanied by an alternate declaring the same fields as one `sysml` fence.
- Each skeleton SHALL carry at least one `ocl` fence under its own clause heading, repeating no clause id.
- No skeleton SHALL carry a placeholder body.
- Each negative fixture SHALL declare an `expect` identifier and a `because` reason.
- No two negative fixtures SHALL declare the same `expect` identifier.
- Each legacy fixture SHALL use the pre-contract free-text Properties form.
- The engine SHALL extract each skeleton to a declaration that validates against that type's emitted schema.
- The engine SHALL extract a skeleton and its alternate to the same fields.
- The engine SHALL report no error for a legacy fixture while `legacy_forms` is `warning`.
- Each negative fixture SHALL be refused, by the engine or by the emitted schema its type names.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-003-AC-1 | Every exported type has a typed-table skeleton with at least one row. | Test |
| FR-003-AC-2 | Every skeleton's `sysml` alternate declares the same field names, types and multiplicities. | Test |
| FR-003-AC-3 | Every skeleton carries at least one `ocl` clause under its own heading, with distinct clause ids. | Test |
| FR-003-AC-4 | No skeleton body carries a placeholder token. | Test |
| FR-003-AC-5 | Every negative fixture declares an `expect` and a `because`, and no two share an `expect`. | Test |
| FR-003-AC-6 | The both-forms fixture carries both Properties forms in one document. | Test |
| FR-003-AC-7 | Every legacy fixture uses the pre-contract free-text form. | Test |
| FR-003-AC-8 | Every skeleton's frontmatter names a type the manifest declares. | Test |
| FR-003-AC-9 | Every skeleton extracts with no error and validates against its emitted schema. | Test |
| FR-003-AC-10 | A skeleton and its alternate extract to identical fields. | Test |
| FR-003-AC-11 | Every legacy fixture extracts with no error under `legacy_forms: warning`. | Test |
| FR-003-AC-12 | Every mapped model names a schema, a skeleton and a golden record that all exist. | Test |
| FR-003-AC-13 | Every golden record equals what its skeleton extracts to. | Test |
| FR-003-AC-14 | Every golden record uses the declared serialization. | Test |
| FR-003-AC-15 | Every negative fixture is refused, by the engine or by its emitted schema. | Test |

## Dependencies

- **Upstream**: [FR-002](./FR-002-emit-schemas-deterministically.md)
- **Downstream**: [NFR-001](../non-functional/NFR-001-verification-without-skips.md)
