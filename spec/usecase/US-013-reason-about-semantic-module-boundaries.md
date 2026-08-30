---
id: US-013
title: "Reason about semantic modules without confusing definitions and projections"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-047"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-048"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-050"
    type: "traces_to"
---

# US-013: Reason about semantic modules without confusing definitions and projections

## Story

**As a** maintainer or consumer of Quoin, Quire, or a semantic module
**I want** one durable account of semantic data planes, authority, and subsystem ownership
**So that** I can extend the ecosystem without turning Markdown, generated code, runtime rows,
or transport bytes into competing sources of truth.

## Context

The ecosystem currently serves both open, dynamically installed modules and finite generated
packages. Its authored specifications and architecture records are Markdown-centric, while
applications also consume extracted data, database state, API payloads, run records, and reports.
Those representations are valuable for different purposes; ambiguity arises only when a projection
is mistaken for its source or when one subsystem silently takes ownership from another.

This story records the boundary before compiler, package-publication, or migration work begins.

## Acceptance Examples (Illustrative)

### US-013-EX-1: An authored requirement and a run are classified differently

- **Given** a typed Markdown requirement and a verification run that refers to it
- **When** a maintainer consults the architecture record
- **Then** the requirement is definition-plane authored knowledge, the run is an
  execution/observation occurrence, and a run report is a presentation projection

### US-013-EX-2: A new module remains usable without native generation

- **Given** a dynamically installed module unknown when a static consumer was compiled
- **When** Quire validates and extracts an artifact from that module
- **Then** the open data remains namespaced and usable without pretending it is one of the
  consumer's finite native generated types

### US-013-EX-3: A schema candidate does not silently become authority

- **Given** the completed TypeSpec feasibility spike recommends the modular JSON Schema fallback
  while ADR-0004 remains provisional
- **When** a maintainer reads the Quoin architecture record
- **Then** TypeSpec is not described as accepted, generated types are not described as authoritative,
  and any promotion still requires its named human decision

### US-013-EX-4: Existing Quire decisions remain coherent

- **Given** Quire's canonical-Markdown boundary, unified archetype shape, and rendering removal
- **When** those decisions are reconciled with the semantic-data architecture
- **Then** Quire remains the parser, validator, extractor, addressor, and byte-splicer rather than
  becoming a template renderer or cross-language generator

## Dependencies

- **Upstream**: [StR-002](../stakeholder/StR-002-extensible-vocabulary.md),
  [StR-003](../stakeholder/StR-003-shared-catalog.md), and
  [StR-004](../stakeholder/StR-004-governed-workflows.md)
- **Downstream**: [FR-046](../functional/FR-046-record-semantic-data-planes.md) through
  [FR-050](../functional/FR-050-reconcile-quire-decisions.md)
