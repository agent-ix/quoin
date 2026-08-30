---
id: SR-043
title: "Scope-boundary review of issue 289 semantic-module architecture requirements"
type: SpecReview
analysis: scope-boundary
scope: "US-013, FR-046..FR-050, NFR-013..NFR-014"
review_set: all
---

# Scope-boundary review of issue 289 semantic-module architecture requirements

## Summary

The specified system is the durable Quoin-side semantic-module architecture record. It allocates
responsibility across repositories but changes only Quoin documentation, requirements, tests,
reviews, and plans.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-034 | low | The record has a closed implementation boundary and an explicit external authority boundary; no parser, compiler, schema, manifest, publication, persistence, or migration work is in scope. | FR-046 Constraints; FR-050 Constraints; NFR-014 |

## Boundary allocation

| Boundary | In issue #289 | Outside issue #289 |
| --- | --- | --- |
| Quoin repository | Architecture index/records/ADRs, requirements, matrix, tests, reviews, plan | Catalog behavior, manifest schema, install/update behavior, evidence execution changes |
| Quire repository | Read and cite current specification and ADRs | Edit parser/extractor/validator/byte-splice behavior or ADR status |
| `filament-core-data` | Read and cite merged architecture and feasibility evidence | Implement compiler/emitters, promote TypeSpec, publish packages |
| Module repositories | Describe their ownership and future mapping concerns | Change vocabularies, manifests, skeletons, schemas, or versions |
| Consumers | Describe adapter/persistence/presentation ownership | Change APIs, databases, migrations, generated bindings, or UIs |
