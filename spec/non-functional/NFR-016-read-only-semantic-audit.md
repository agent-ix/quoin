---
id: NFR-016
title: "The semantic audit is read-only and non-disruptive"
type: NFR
quality_attribute: compatibility
relationships:
  - target: "ix://agent-ix/quoin/FR-051"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-055"
    type: "constrains"
  - target: "ix://agent-ix/quoin/NFR-014"
    type: "references"
---

# NFR-016: The semantic audit is read-only and non-disruptive

> **⛔ Withdrawn — 2026-09-29.** This requirement constrained the semantic
> type-fit audit, which is withdrawn with US-014 and FR-051..FR-055 under
> [quoin#654](https://github.com/agent-ix/quoin/pull/654). The audit's generator
> and test were removed by the native cutover
> ([quoin#524](https://github.com/agent-ix/quoin/pull/524)) and its retained
> output is deleted by quoin#654, so the thing it constrained no longer exists
> and nothing surviving inherits the obligation.
>
> The text below is retained unchanged. It describes what was required, not what
> is required.

## Statement

The semantic type-fit audit SHALL inspect and report without changing module sources, installed
registries, schemas, skeletons, generated packages, runtime behavior, persistence, or consumer contracts.

## Scope

- Applies to issue #288 and its specification, audit implementation, retained analysis, review, and plan artifacts.
- Allows deterministic audit code and generated review artifacts inside Quoin.
- Allows the exact root formatter-ignore entry required to preserve content-addressed generated audit bytes.
- Excludes compiler, code-generation, schema evolution, publication, migration, enforcement, and retirement work.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Writes outside the configured audit output directory | 0 | 0 | Test (TC-1192) |
| Production source, module manifest, schema, skeleton, registry, generated package, migration, or consumer contract files changed | 0 | 0 | Test (TC-1193) |
| Major-interference recommendation advanced without its named gate | 0 | 0 | Inspection (TC-1194) |

## Verification

Tests run the audit against read-only fixture inputs and compare source-tree state before and after.
A changed-path guard constrains the pull request. The review stops at any recommendation that would
activate a breaking contract, data migration, publication, enforcement, retirement, or conflicting
feature-work change.

## Dependencies

- **Upstream**: [FR-051](../functional/FR-051-snapshot-semantic-audit-scope.md),
  [FR-055](../functional/FR-055-reconcile-semantic-audit-findings.md),
  [NFR-014](./NFR-014-non-disruptive-architecture-record.md)
