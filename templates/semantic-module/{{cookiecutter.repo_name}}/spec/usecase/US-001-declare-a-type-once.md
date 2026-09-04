---
id: US-001
title: "Declare a type once and have every surface read it"
type: US
relationships:
  - target: "ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/StR-001"
    type: "traces_to"
---

# US-001: Declare a type once and have every surface read it

## Story

**As a** maintainer of {{ cookiecutter.repo_name }}
**I want** to declare each type's fields, relations, operations and invariants once, in one source
**So that** the emitted schema, the authoring skeleton and the validator agree without my keeping three copies in step by hand.

## Context

The module's structural source is `typespec/main.tsp`. Everything under
`{{ cookiecutter.package_name }}/schemas/` is produced from it, and the manifest
references those bytes by digest. The authoring skeletons show the Markdown forms
that populate a declaration, and the fixtures show what is refused.

## Acceptance Examples (Illustrative)

### US-001-EX-1: One source, one schema

- **Given** a type declared in `typespec/main.tsp`
- **When** `make schemas` runs
- **Then** one JSON Schema is written for it and the manifest's digest for that type is rewritten to match the bytes

### US-001-EX-2: The two authoring forms agree

- **Given** a skeleton with a typed `## Properties` table and its `sysml`-fence alternate
- **When** both are extracted
- **Then** the two declarations are identical

### US-001-EX-3: A refusal is exercised, not asserted

- **Given** a fixture that violates one rule the emitted schema enforces
- **When** the suite runs
- **Then** that fixture is refused, and no two fixtures are refused for the same declared reason

## Constraints (Contextual)

The emitted schemas are never hand-edited and the digests are never typed. The
suite fails rather than skips when a tool it needs is absent.

## Priority and Risk (Informative)

P0. The risk if unmet is a module whose published schema and whose documented
authoring form describe different things.

## Traceability (Informative)

Drives [FR-001](../functional/FR-001-declare-the-semantic-contract.md),
[FR-002](../functional/FR-002-emit-schemas-deterministically.md) and
[FR-003](../functional/FR-003-authoring-forms-and-fixtures.md), constrained by
[NFR-001](../non-functional/NFR-001-verification-without-skips.md).
