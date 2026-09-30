---
id: FR-055
title: "Reconcile audit findings with current semantic contracts"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-014"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-050"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-054"
    type: "depends_on"
---

# FR-055: Reconcile audit findings with current semantic contracts

> **⛔ Withdrawn — 2026-09-29.** This requirement is withdrawn and is not
> implemented, with the rest of the semantic type-fit audit (US-014, FR-051..FR-055,
> NFR-015, NFR-016) under [quoin#654](https://github.com/agent-ix/quoin/pull/654).
> The audit ran once under [quoin#288](https://github.com/agent-ix/quoin/issues/288);
> its generator and test were removed by the native cutover
> ([quoin#524](https://github.com/agent-ix/quoin/pull/524)), and its retained output
> is deleted by quoin#654. Nothing in quoin produces or reads the audit any more.
>
> The text below is retained unchanged. It describes what was required, not what
> is required.

## Description

When the semantic audit is submitted for acceptance, it SHALL reconcile every proposed conflict, missing type,
and repository impact with the governing architecture record, the Quire corpus decision, and the
Filament core-data contract census.

## Rationale

An audit that invents a parallel vocabulary or treats a provisional compiler decision as accepted
would create the inconsistency the semantic-data program is intended to remove.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-055-AC-1 | Every conflict and missing-type row cites the applicable data plane, authority-by-concern entry, subsystem owner, and governing or provisional decision. | Test (TC-1183) |
| FR-055-AC-2 | Every overlapping concept in the `filament-core-data#10` census is linked and classified as reuse, extension, mapping, conflict, or unrelated; the audit creates no shadow core contract. | Test (TC-1184) |
| FR-055-AC-3 | Every Quire parsing, identity, addressing, validation, extraction, or splicing implication cites the pinned `quire-rs#385` evidence and preserves Quire's recorded boundary. | Test (TC-1185) |
| FR-055-AC-4 | Proposed follow-up boundaries distinguish analysis from compiler, code-generation, module-schema, migration, database, API, publication, enforcement, and retirement work, and mark every major-interference boundary as gated. | Test (TC-1186) |
| FR-055-AC-5 | Acceptance uses a fresh census of `default-modules.yaml` and all resolved module heads; drift after the recorded snapshot makes the review stale and blocks signoff. | Test (TC-1187) |

## Constraints

- Reconciliation records recommendations, not authorization to implement them.
- Provisional decisions remain provisional until their named human gate is satisfied.

## Dependencies

- **Upstream**: [FR-054](./FR-054-publish-semantic-audit-artifacts.md),
  [FR-046](./FR-046-record-semantic-data-planes.md) through
  [FR-050](./FR-050-reconcile-quire-decisions.md)
- **External**: `agent-ix/quire-rs#385`, `agent-ix/filament-core-data#10`
