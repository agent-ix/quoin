---
id: FR-124
title: "Inspect installed semantic modules and their import locks"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-075"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-049"
    type: "depends_on"
---

# FR-124: Inspect installed semantic modules and their import locks

## Description

`quoin module inspect` SHALL report, for every installed module, the semantic
contract it declares and whether its import locks still hold against the
installed set, so that dynamic-only and generated-package modules are visible in
one catalog and a stale lock is found where it can be fixed.

## Rationale

`semantic.imports` maps an imported package identity to an exact version: that
mapping is the module's lock on the semantic packages it builds on (FR-070,
FR-075). The install gate judges it once, when the importing module is
installed. A module installed later, or reinstalled at a new version, changes
what the installed set provides without re-judging the importer
(`validate_installed` deliberately re-checks only each module's own block), so a
lock can go stale silently. Nothing else reports it. The same command answers
which installed modules are consumed dynamically only and which also declare a
generated package target (FR-049).

A recorded source revision, schema fingerprint, compiler version or artifact
digest is not part of this report: the registry records the requested `ref`, and
the module's own `version` and exact imports are the whole lock. Recording more
would be a provenance ledger that nothing reads (the owner's removal of digests
and pins, #716). Generated-package coordinates and publication stay with
`agent-ix/quoin#290`; Quoin installs from a git location and never fetches a
generated package (FR-075-CON-1), so there is no generated-package substitution
to inspect.

## Behavior

- `quoin module inspect` SHALL write one JSON document `{ "modules": [...] }` with one entry per installed module, in registry order.
- Each entry SHALL carry `name`, `mode`, `diagnostics`, the registry `ref` when one was recorded, and, when the module's `semantic` block validated, its `version` and the block itself.
- `mode` SHALL be `no-semantic-block` when the module has no `semantic` block or its block was refused, `compiled` when `semantic.targets` declares any of `rust`, `typescript`, `python-pydantic-v2`, or `python-dataclass`, and `dynamic` otherwise.
- An entry's `diagnostics` SHALL be the diagnostics of reading its block (FR-070, FR-073, FR-074) followed by one error per `semantic.imports` entry that no other installed module provides at exactly the locked version, naming the import, the locked version and the installed versions, and one error per import cycle.
- Where a block names a contract version this build does not define, `inspect` SHALL report the refusal as a diagnostic on that module.
- Where a block names a contract version this build does not define, `inspect` SHALL report no `semantic` view for that module.
- If any entry carries an error-severity diagnostic, then `inspect` SHALL still write the complete document.
- If any entry carries an error-severity diagnostic, then `inspect` SHALL exit with the partial status (1) and name each failing module on stderr.
- `inspect` SHALL list a module with no `semantic` block.
- `inspect` SHALL exit zero when the only findings are modules with no `semantic` block.
- `inspect` SHALL leave the registry, every installed module and every consuming repository unmodified.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-124-AC-1 | With a dynamic-only module and a module declaring a `rust` target installed together, `inspect` reports `dynamic` for the first and `compiled` for the second, each with its package and contract version. | Test |
| FR-124-AC-2 | A clean installed set exits zero and every entry's error diagnostics are empty. | Test |
| FR-124-AC-3 | A module with no `semantic` block is listed as `no-semantic-block` and the command exits zero. | Test |
| FR-124-AC-4 | When an imported module is reinstalled at a version other than the locked one, `inspect` exits 1, writes the whole document, and reports on the importer an error naming the import, the locked version and the installed version. | Test |
| FR-124-AC-5 | A block whose `contract_version` is `2.0.0` is reported with an error diagnostic naming `2.0.0` and no `semantic` view, and the command exits 1. | Test |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md), [FR-075](./FR-075-semantic-package-exports-and-locks.md), [FR-049](./FR-049-preserve-dynamic-and-generated-modules.md), [FR-019](./FR-019-manage-plugin-registry.md)
- **Downstream**: `agent-ix/quoin#290` (publication)
