---
id: NFR-001
title: "Default-module reconciliation is idempotent and offline-safe"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-017"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-005"
    type: "traces_to"
---

# NFR-001: Default-module reconciliation is idempotent and offline-safe

## Statement

Default-module reconciliation SHALL be idempotent and SHALL perform no network or
git work once each module is installed and pinned, so that repeated `catalog` and
`write` invocations remain offline-safe.

## Scope

- Applies to: the lazy default-module reconcile triggered on `catalog`/`write`
  access and via `plugin ensure-defaults`.
- Operational context: steady-state authoring after the first install.

## Rationale

Authors run the write-validate loop many times per session, often offline.
Re-fetching modules on every access would be slow and could change behaviour when
an upstream module moved; an idempotent, offline reconcile keeps the loop fast and
reproducible.

## Measurement and Evaluation

| Metric                                                         | Target | Threshold | Method |
| -------------------------------------------------------------- | ------ | --------- | ------ |
| Network/git operations on repeat catalog access after install  | 0      | 0         | Test   |
| Module-store mutations on repeat reconcile of an installed set | 0      | 0         | Test   |

## Verification

The default-module reconcile is run twice against an already-installed set and
asserted to perform no further install or git work, confirmed by the
`index.test.ts` lazy-install test, which reconciles path-source fixtures with no
network or git.

## Dependencies

- **Upstream**: [FR-017](../functional/FR-017-reconcile-default-modules.md), whose
  behaviour this constrains.
- **Downstream**: live first-install-then-offline behaviour is verified by
  [IT-001](../integration/IT-001-default-module-reconcile.md).
