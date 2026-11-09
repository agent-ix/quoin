---
id: FR-013
title: "Resolve the requested types for an authoring pack"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-001"
    type: "implements"
---

# FR-013: Resolve the requested types for an authoring pack

## Description

The `quoin write <repo_dir> --types <type[,type...]>` command SHALL require at
least one requested type and a `repo_dir` that exists and is a directory, SHALL
split the type list on commas while trimming whitespace and dropping empty
entries, and SHALL raise an error listing the available types in sorted order
when a requested type is absent from the catalog.

## Inputs

- A `repo_dir` positional and one or more `--types` values.

## Outputs

- An ordered list of resolved catalog entries, or a raised error.

## Behavior

- The command SHALL raise an error when no type is requested.
- The command SHALL raise an error when `repo_dir` is missing, does not exist, or
  is not a directory.
- The command SHALL split type values on commas, trim each, and drop empties.
- The command SHALL resolve each requested type against the catalog
  case-insensitively, raising an error that lists the available types sorted
  alphabetically when one is not found.

## Acceptance Criteria

| ID          | Criteria                                                                       | Verification                      |
| ----------- | ------------------------------------------------------------------------------ | --------------------------------- |
| FR-013-AC-1 | An empty type list raises an error                                             | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-013-AC-2 | A missing `repo_dir`, a non-existent path, and a file path each raise an error | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-013-AC-3 | Type values are comma-split, trimmed, and emptied entries dropped              | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-013-AC-4 | An unknown type raises an error listing available types in sorted order        | None (no native test is tagged to this criterion; PLAT-1045) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-001](../usecase/US-001-author-root-artifact-with-objects.md). Resolution uses
  [FR-010](./FR-010-case-insensitive-type-lookup.md).
- **Downstream**: resolved entries are rendered by
  [FR-014](./FR-014-emit-authoring-contract.md).
