---
id: FR-012
title: "Detect duplicate type definitions and validate the catalog"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-006"
    type: "implements"
---

# FR-012: Detect duplicate type definitions and validate the catalog

## Description

The catalog loader SHALL flag any type name declared as the same kind by more
than one module as a duplicate, and the `catalog validate` command SHALL report a
clean catalog with its module count (text or JSON), or exit with a non-zero
status emitting `{ok: false, duplicates}` on standard error when duplicates
exist.

## Inputs

- The assembled catalog and a `catalog validate` invocation with an optional
  `--json` flag.

## Outputs

- A success report with the module count, or a duplicates report on standard
  error with a non-zero exit status.

## Behavior

- The loader SHALL group catalog entries by kind and name and SHALL flag any
  name held by more than one module, listing the declaring modules in sorted
  order.
- `catalog validate` SHALL print the module count (text or JSON) when no
  duplicates exist.
- `catalog validate` SHALL emit `{ok: false, duplicates}` on standard error and
  set a non-zero exit status when duplicates exist.

## Acceptance Criteria

| ID          | Criteria                                                                                    | Verification           |
| ----------- | ------------------------------------------------------------------------------------------- | ---------------------- |
| FR-012-AC-1 | A type declared by two modules is reported as a duplicate with the declaring modules sorted | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-012-AC-2 | A catalog with unique type names reports no duplicates                                      | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-012-AC-3 | `catalog validate` reports the module count when clean (text and JSON)                      | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-012-AC-4 | `catalog validate` emits `{ok:false,duplicates}` on stderr and exits non-zero on conflict   | None (no native test is tagged to this criterion; PLAT-1045) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-006](../usecase/US-006-detect-conflicting-type-definitions.md).
- **Downstream**: supports the actionable-failure guarantee in
  [NFR-003](../non-functional/NFR-003-actionable-errors.md).
