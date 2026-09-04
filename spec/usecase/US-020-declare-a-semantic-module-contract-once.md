---
id: US-020
title: "Declare a module's semantic contract once"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-013"
    type: "depends_on"
---

# US-020: Declare a module's semantic contract once

## Story

**As a** maintainer of a Quire object module
**I want** to declare my archetypes' typed fields, relations, operations, and invariants once, in the spec artifact, against the shared semantic-core grammar
**So that** Quire validation, the extraction frontend, the generated packages, and the formal-clause checkers all read one declaration instead of prose I have to keep in sync by hand.

## Context

Today every object archetype declares `data_schema: {type: object}` and extracts
its Properties section as free text. Field types live in prose (`UUID (optional)`,
`Dict[str, Any]`), so nothing downstream can type a field. The semantic-core L3
grammar (`agent-ix/filament-core-data#35`) and IR v1.1
(`agent-ix/filament-core-data#34`) now exist; this story is the Quoin side: what a
module manifest declares and how a spec artifact's Markdown maps onto the
grammar. Quire (`agent-ix/quire-rs#388`) implements the extraction, recognising the
`sysml` subset at line level and never parsing expressions; this story fixes the
contract it implements.

## Acceptance Examples (Illustrative)

### US-020-EX-1: A typed table becomes a declaration set

- **Given** an entity FR with a `## Properties` table whose columns are `Field | Type | Multiplicity | Constraints`
- **When** Quire extracts the artifact under a module that references the semantic-core `FieldDecl.json`
- **Then** the extraction yields one `FieldDecl` per row, with `Type` resolved to a kernel scalar, an enumeration, or a declared object, and `Constraints` parsed into the closed keyword vocabulary

### US-020-EX-2: A SysML fence says the same thing

- **Given** the same artifact authored with a ```` ```sysml ```` fence under `## Properties` instead of the table
- **When** both artifacts are extracted
- **Then** the two `FieldDecl[]` results are identical, and an artifact carrying both forms is rejected

### US-020-EX-3: A module points at its emitted schema

- **Given** a module manifest whose `entity.data_schema` names the emitted `Entity.json` by path and digest and records the semantic-core version it imports
- **When** the manifest is loaded
- **Then** the schema is read from the path, the digest is verified, and a mismatch is an error rather than a silent fallback to `{type: object}`

### US-020-EX-4: A legacy artifact keeps validating

- **Given** an existing corpus artifact with a bullet-list Properties section
- **When** it is validated under the new module version
- **Then** validation passes with a `warning` naming the migration, and no corpus file is edited

## Options (Exploratory)

The manifest could embed the grammar, reference emitted schemas, or point at a
generated package. Table and fence could be alternatives or the fence could be a
rendering of the table. Legacy forms could be errors, warnings, or silently
accepted. The functional requirements settle each.

## Constraints (Contextual)

Quire stays parser, validator, extractor, and byte-splicer; it never parses fence
content and never renders. No corpus repository is edited by this campaign.
No current manifest becomes invalid before the advisory sweep and promotion gate.

## Dependencies (Contextual)

Depends on [US-013](./US-013-reason-about-semantic-module-boundaries.md), the
semantic-core grammar (`agent-ix/filament-core-data#35`), and IR v1.1
(`agent-ix/filament-core-data#34`). Blocks `agent-ix/quire-rs#388` and every
Wave 4 module ticket.

## Priority and Risk (Informative)

P0. The principal risk is two authoring forms that drift apart, or a manifest
extension that quietly invalidates the installed corpus.

## Traceability (Informative)

Drives [FR-070](../functional/FR-070-semantic-module-manifest-extension.md)
through [FR-075](../functional/FR-075-semantic-package-exports-and-locks.md),
constrained by [NFR-017](../non-functional/NFR-017-non-disruptive-manifest-evolution.md).
