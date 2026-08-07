---
id: StR-002
title: "Authors extend the spec vocabulary with community modules"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-018"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-019"
    type: "satisfied_by"
---

# StR-002: Authors extend the spec vocabulary with community modules

## Stakeholder Need

Teams adopting spec-driven development require that `quoin` shall let them add
their own and community-published artifact and object types on top of the default
set, installing a module from a local path or a GitHub repository so that its
types participate in authoring and validation exactly like the built-in types.

## Rationale

No fixed vocabulary fits every domain. A team modelling, for example, hardware
interfaces or a regulated workflow needs artifact and object types the default
modules do not ship. If the vocabulary were closed, those teams could not express
their domain in the spec at all. Allowing extension through installable modules
keeps the system open and lets a community of modules grow around it, while the
author keeps one consistent authoring and validation flow.

## Validation Criteria


| ID | Criteria | Validation |
|----|----------|------------|
| StR-002-VC-1 | An author installs a module from a supported source, sees its types appear in the active catalog, requests those types in an authoring contract, and validates files authored against them — with the installed module treated identically to a default module. | Demonstration |

Satisfaction is demonstrated by installing a module from a path and from a GitHub source and authoring against its types.

## Stakeholders

The primary stakeholders are teams with domain-specific vocabularies and the
authors of community modules; affected parties are downstream consumers who read
specs authored against the extended vocabulary.

## Dependencies

**Downstream**: the plugin source-mapping and registry functional requirements
([FR-018](../functional/FR-018-map-plugin-sources.md),
[FR-019](../functional/FR-019-manage-plugin-registry.md)). Install and reconcile
mechanics are provided by `@agent-ix/ts-plugin-kit`.
