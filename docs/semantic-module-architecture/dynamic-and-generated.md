---
id: ARCH-SM-004
title: "Dynamic modules and finite generated packages"
status: proposed
requirements:
  - FR-049
---

# Dynamic modules and finite generated packages

Dynamic module discovery and native generated packages solve different consumption problems.
The ecosystem preserves both.

## Two compatible consumption profiles

### Open dynamic profile

Quoin may install a previously unknown module after the Quire or consumer binary was built.
Quire loads the module contract, validates its typed Markdown, and preserves its namespaced
semantic identity and generic validated data. A dynamic consumer can search, graph, transmit,
or present that data without claiming a native compile-time type it does not have.

Core routing, identity, provenance, and typed outcome fields remain in a small stable kernel.
Module-specific extensions remain namespaced and versioned.

### Finite static profile

A static consumer selects a finite generated export set. Each selected semantic module becomes
an independently versioned package with native Rust, TypeScript, and Python types, runtime
validation where the target requires it, schemas, and versioned package dependencies on the
small semantic core and imported module packages.

Finite generation improves ergonomics, compilation, and boundary safety. It does not declare
that every module the ecosystem can load was known when that consumer was compiled.

## Unknown-extension policy

Every static profile declares how an unknown extension is handled. The permitted outcomes are
preserve, reject, or surface:

- **preserve** it as opaque or generic namespaced validated data for round-trip safety;
- **reject** it with a typed unsupported-module/version outcome at a closed boundary; or
- **surface** it explicitly to a plugin or generic UI path.

The selected behavior must appear in a named profile. The consumer must never misclassify an
unknown extension as a known native type, silently discard it, or infer its semantic role from
record shape alone.

## Regeneration and adoption

Installing a dynamic module does not require regeneration of every static consumer. A static
consumer regenerates only when it elects to adopt that module's native package or changes its
selected export/profile. Until then, the profile's unknown-extension policy governs.

Regeneration is deterministic from accepted source, package metadata, profile, mappings,
compiler version, and emitter version. It is a proposed diff, never a silent overwrite of
consumer-owned code.

## Declaration separation

| Declaration     | Own concern                                                                                       | Must not imply                                 |
| --------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| Module manifest | catalog and distribution identity, version, dependencies, artifacts/objects, and install metadata | Native package generation or consumer adoption |
| Exports         | Public semantic types in a generated package                                                      | A transport or Markdown layout                 |
| Targets         | Rust, TypeScript, Python, JSON Schema, wire, or analytical outputs                                | Field mappings for every target                |
| Mappings        | Correspondence between semantic structure and one representation                                  | That the representation is authoritative       |
| Profiles        | Selected exports, targets, mappings, options, and compatibility posture                           | A global default for all consumers             |

Catalog manifests, exports, targets, mappings, and profiles are distinct concerns. They may be
referenced together by a build, but selecting “Python” does not define Markdown headings,
PostgreSQL columns, Protobuf field numbers, or Arrow loss rules.

## Current-state protection

This model adds no required manifest field, removes no module, and requires no generated package
today. Existing skeletons, schemas, direct typed-Markdown authoring, Quoin discovery/installation,
and Quire validation/extraction remain valid. Compiler and package work begins only in separately
activated tickets with its own compatibility and publication gates.

## Traceability

This record implements FR-049 and specializes `filament-core-data` ARCH-005 and ARCH-006.
See [ADR-0001](adr/0001-authority-by-concern.md) for why generation does not become authority.
