---
id: FR-049
title: "Preserve dynamic modules and finite generated packages"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-013"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-016"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-019"
    type: "references"
---

# FR-049: Preserve dynamic modules and finite generated packages

## Description

When dynamic modules and generated semantic packages coexist, the architecture record SHALL define
their compatibility without closing Quoin and Quire to only types known at consumer compile time.

## Rationale

Quoin and Quire intentionally discover module vocabularies at runtime. Rust, TypeScript, and Python
consumers also benefit from finite native generated types. These are complementary consumption
profiles, not mutually exclusive architectures.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-049-AC-1 | A dynamic consumer may load a previously unknown namespaced module and preserve its validated data through generic open-value surfaces. | Test (TC-1140) |
| FR-049-AC-2 | A static consumer receives a finite, versioned export set with native generated types and explicit package dependencies. | Test (TC-1141) |
| FR-049-AC-3 | An unknown extension is preserved, rejected, or surfaced according to a named profile and is never misclassified as a known native type. | Test (TC-1142) |
| FR-049-AC-4 | Installing a new dynamic module does not require regeneration unless a static consumer elects to adopt that module's native package. | Test (TC-1143) |
| FR-049-AC-5 | Module manifests retain catalog/distribution concerns while package exports, generation targets, and representation mappings remain distinct declarative concerns. | Test (TC-1144) |

## Constraints

- Existing manifest and catalog behavior remains valid; this record adds no required manifest field.
- Generated packages do not replace module skeletons, schemas, or direct typed-Markdown authoring.

## Dependencies

- **Upstream**: [US-013](../usecase/US-013-reason-about-semantic-module-boundaries.md),
  [FR-016](./FR-016-default-modules-schema.md), [FR-019](./FR-019-manage-plugin-registry.md)
- **External basis**: `filament-core-data` ARCH-005 and ARCH-006
