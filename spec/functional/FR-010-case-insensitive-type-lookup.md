---
id: FR-010
title: "Look up catalog types case-insensitively"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-002"
    type: "implements"
---

# FR-010: Look up catalog types case-insensitively

## Description

The catalog SHALL resolve a type by name case-insensitively, so that requests
such as `FR` and `fr` resolve to the same catalog entry.

## Inputs

- A type name in any letter case.

## Outputs

- The matching catalog entry, independent of the input case.

## Behavior

- The catalog SHALL normalize both the requested name and each entry's name to a
  common case before comparison.
- A match SHALL return that entry regardless of the case used in the request.

## Acceptance Criteria

| ID          | Criteria                                                        | Verification         |
| ----------- | --------------------------------------------------------------- | -------------------- |
| FR-010-AC-1 | `FR` and `fr` resolve to the same artifact entry                | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-010-AC-2 | A mixed-case request such as `DOMAIN` resolves the object entry | None (no native test is tagged to this criterion; PLAT-1045) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-002](../usecase/US-002-discover-authoring-contract.md).
- **Downstream**: case-insensitive lookup is relied on by
  [FR-011](./FR-011-list-and-show-catalog.md) and
  [FR-013](./FR-013-resolve-requested-types.md).
