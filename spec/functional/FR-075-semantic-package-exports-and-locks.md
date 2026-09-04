---
id: FR-075
title: "Semantic package exports, imports, locks, and generated coordinates"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "depends_on"
---

# FR-075: Semantic package exports, imports, locks, and generated coordinates

## Description

When a module declares a `semantic` block, Quoin SHALL derive the module's
`filament-core-data` package manifest and lock entries (imports, exports, generated
package coordinates, schema fingerprint, compatibility posture) from the manifest
and the installed catalog, so that dynamic use and static generated use share one
identity graph.

## Rationale

Ticket #293 deliverable three and FR-049: dynamic modules and finite generated
packages coexist over one identity graph; the lock is where that graph is pinned.

## Behavior

- Quoin SHALL emit, for each semantic module, a `package-manifest` document conforming to `filament-core-data` `package-manifest.schema.json` with `package.identity` from `semantic.package`, `imports` from `semantic.semantic_core` and any imported modules, `exports` from `semantic.exports`, and `targets` from `semantic.targets`.
- Quoin SHALL record the emitted-schema digests of every exported object type in the catalog lock alongside the module version, forming the module's schema fingerprint.
- When a module imports another module's semantic package at a version the lock does not contain, `quoin install` SHALL fail naming the import.
- Quoin SHALL expose generated package coordinates (`rust`, `typescript`, `python-pydantic-v2`, `python-dataclass`, `json-schema`) as declarations only; publication stays with `agent-ix/quoin#290`.
- A dynamic consumer SHALL load the module and validate artifacts with no generated package present (FR-049-AC-1).
- A static consumer SHALL find the same identities in the generated package (FR-049-AC-2).
- The `semantic` block SHALL declare the module version's compatibility posture (`strict`, `additive`, `declared-lossy`), which Quoin copies to the package manifest.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-075-CON-1 | Quoin SHALL NOT compile, publish, or fetch generated packages in this specification; it declares coordinates. | Boundary | Inspection |
| FR-075-CON-2 | Package identities SHALL be the `ix://<org>/<repo>` form used by the IR and never a registry URL. | Integrity | Schema validation |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-075-AC-1 | The derived package manifest for a fixture module validates against `filament-core-data` `package-manifest.schema.json`. | Test |
| FR-075-AC-2 | The catalog lock entry carries one digest per exported object type and changes when an emitted schema changes. | Test |
| FR-075-AC-3 | An import at a version absent from the lock fails `quoin install` naming the import. | Test |
| FR-075-AC-4 | The same object-type identities appear in the dynamic load and in the declared generated coordinates. | Test |
| FR-075-AC-5 | A `semantic.package` given as a URL is rejected. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-049](./FR-049-preserve-dynamic-and-generated-modules.md), `filament-core-data` FR-021 (package graphs and locks)
- **Downstream**: `agent-ix/quoin#287` (catalog locks), `agent-ix/quoin#290` (publication), `agent-ix/quoin#292`
