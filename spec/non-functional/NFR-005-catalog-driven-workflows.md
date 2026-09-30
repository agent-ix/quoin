---
id: NFR-005
title: "Workflows reference catalog-defined types"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-020"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
---

# NFR-005: Workflows reference catalog-defined types

## Statement

Workflow definitions SHOULD reference the artifact and object types defined in the
catalog rather than redefining the document or object vocabulary, so that one
catalog remains the single source of type definitions across authoring,
validation, and workflows.

## Scope

- Applies to: the bundled `review`, `matrix`, and `to-plan` workflow definitions.
- Operational context: workflow authoring and maintenance.

## Rationale

If a workflow redefined types, the vocabulary would drift from the catalog and
authors could face two competing definitions of the same type. Referencing the
catalog keeps the vocabulary singular and maintainable.

## Measurement and Evaluation

| Metric                                                              | Target | Threshold | Method |
| ------------------------------------------------------------------- | ------ | --------- | ------ |
| Workflow-defined doc/object types that duplicate catalog vocabulary | 0      | 0         | Inspection |

## Verification

The bundled workflow definitions are reviewed to confirm they reference
catalog-defined types and do not redefine the document or object vocabulary.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-005-AC-1 | No skill under `skills/` teaches a Test Matrix `Status` vocabulary or ships a Test Matrix template; the only statement of that vocabulary is the installed `spec-artifacts-process` manifest's (`traceability.status`). | Inspection |

## Dependencies

- **Upstream**: [FR-020](../functional/FR-020-resolve-workflow-skills.md), whose
  workflow launchers this constrains.
- **Downstream**: none.

> **CR-001 (2026-09-29, PLAT-1083):** AC-1 is restated. It coupled the `Status`
> vocabulary `skills/spec-matrix/SKILL.md` and its two asset templates taught to
> the module manifest. The skill no longer teaches agents to write a matrix and
> its templates are deleted, so the single-vocabulary claim now holds by
> absence: no skill restates the vocabulary at all. The test that compared the
> two copies retired with the Node harness and is not replaced.
