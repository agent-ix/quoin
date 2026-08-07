---
id: StR-005
title: "Authoring stays offline-safe and reproducible"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-016"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-017"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/NFR-001"
    type: "satisfied_by"
---

# StR-005: Authoring stays offline-safe and reproducible

## Stakeholder Need

Authors require that once their module set is in place, repeated spec work shall
run without further network access and shall resolve the same pinned module
versions every time, so that authoring is fast, reproducible, and unaffected by
upstream changes or connectivity.

## Rationale

Authors run the write-validate loop many times per session, often offline or
behind restricted networks. If every catalog access re-fetched modules, the loop
would be slow and could silently change behaviour when an upstream module moved.
Declaring the default set at pinned versions and installing it once gives every
author the same vocabulary deterministically, and keeps subsequent work entirely
local.

## Validation Criteria


| ID | Criteria | Validation |
|----|----------|------------|
| StR-005-VC-1 | The default module set is declared at fixed versions, installed once, and thereafter served from the local store with no network or version-resolution work on repeated catalog access — yielding identical results across runs. | Inspection |

Satisfaction is demonstrated by exercising catalog and authoring commands repeatedly with no network activity after the first install.

## Stakeholders

The primary stakeholders are spec-authoring agents and operators working in
constrained or offline environments; affected parties are CI pipelines that need
reproducible spec tooling.

## Dependencies

**Downstream**: the default-module-set schema and reconcile functional
requirements ([FR-016](../functional/FR-016-default-modules-schema.md),
[FR-017](../functional/FR-017-reconcile-default-modules.md)) and the idempotent,
offline-safe non-functional requirement
([NFR-001](../non-functional/NFR-001-idempotent-offline-reconcile.md)).
