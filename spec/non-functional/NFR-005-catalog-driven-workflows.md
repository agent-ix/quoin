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
| NFR-005-AC-1 | The Status vocabulary that `skills/spec-matrix/SKILL.md` and both of its asset templates teach equals the classed status set the installed `spec-artifacts-process` manifest declares (`traceability.status`), and every taught marker is admitted by the manifest's Status column pattern; divergence in either direction fails the suite. A marker the manifest admits but classes as nothing is deliberately not required to be taught. | Test (TC-271) |

## Dependencies

- **Upstream**: [FR-020](../functional/FR-020-resolve-workflow-skills.md), whose
  workflow launchers this constrains.
- **Downstream**: none.
