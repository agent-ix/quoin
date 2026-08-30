---
id: US-014
title: "Audit the semantic fit of the complete default-module corpus"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-051"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-052"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-053"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-054"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-055"
    type: "traces_to"
---

# US-014: Audit the semantic fit of the complete default-module corpus

## Story

**As a** maintainer planning the semantic-data program
**I want** an evidence-backed census and type-fit review of every default module
**So that** migrations and generators are designed around the corpus we actually have rather than a
representative sample or an assumed type system.

## Context

The architecture record distinguishes semantic definitions, runtime occurrences, and projections,
but the installed module ecosystem predates that terminology. Before changing manifests, schemas,
Markdown contracts, generated packages, or consumers, the current corpus must be measured at exact
revisions. The audit is deliberately read-only: it records fit, conflicts, missing types, and likely
repository impact without promoting a schema or modifying a module.

## Acceptance Examples (Illustrative)

### US-014-EX-1: A placeholder schema remains visible

- **Given** a declared type whose schema validates any object
- **When** the audit evaluates generated-code suitability
- **Then** the type is recorded as incomplete with the schema evidence, not counted as schema-backed

### US-014-EX-2: A document cannot be parsed

- **Given** a Markdown file within a pinned module corpus
- **When** Quire cannot parse or resolve it
- **Then** the inventory retains the file and its failure classification instead of shrinking the denominator

### US-014-EX-3: A definition-shaped type carries occurrence data

- **Given** a report type that accumulates run, result, timestamp, or evidence fields
- **When** the type is scored against the four-plane architecture
- **Then** the review records the definition/occurrence conflict and the missing semantic type, if any

### US-014-EX-4: A repository revision drifts

- **Given** a requested module ref, a resolved commit, and installed bytes that disagree
- **When** the audit is produced
- **Then** the disagreement is a blocking provenance record and is never normalized away

## Dependencies

- **Upstream**: [US-013](./US-013-reason-about-semantic-module-boundaries.md) and its architecture record
- **Downstream**: [FR-051](../functional/FR-051-snapshot-semantic-audit-scope.md) through
  [FR-055](../functional/FR-055-reconcile-semantic-audit-findings.md)
- **External basis**: `agent-ix/quire-rs#385` corpus, `agent-ix/filament-core-data#10`
