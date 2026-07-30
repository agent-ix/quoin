---
id: FR-009
title: "Read each module manifest into catalog entries"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-002"
    type: "implements"
---

# FR-009: Read each module manifest into catalog entries

## Description

For each module the loader SHALL read the module `name` (defaulting to the
directory basename), the optional `version`, and the `artifact_types` and
`object_types` entries, ignoring malformed entries; for each artifact type it
SHALL resolve a `frontmatter_schema_ref` to a schema path, and for every type it
SHALL resolve a skeleton from `skeletons/<Type>.md`, falling back to a
lowercase-filename variant.

## Inputs

- A module root containing `manifest.yaml`, its `skeletons/`, and its `schemas/`.

## Outputs

- A module record (name, version, type names) and one catalog entry per resolved
  artifact and object type, each carrying its module, root, and resolved
  skeleton/schema paths.

## Behavior

- The loader SHALL read `name`, defaulting to the directory basename when absent.
- The loader SHALL read `version` when present and leave it unset otherwise.
- The loader SHALL include only `artifact_types`/`object_types` list entries that
  are objects carrying a `name`, ignoring non-array and malformed entries.
- For an artifact type, the loader SHALL resolve `frontmatter_schema_ref` against
  the module root into a schema path when present.
- For every type, the loader SHALL resolve a skeleton at `skeletons/<Type>.md`,
  then at the lowercase-filename variant, leaving it unset when neither exists.
- The loader SHALL match those two filenames against the directory's actual
  entries, so a resolved path always names a file as it exists on disk and no
  other casing resolves, whether or not the filesystem is case-sensitive.

## Acceptance Criteria

| ID          | Criteria                                                                                           | Verification                        |
| ----------- | -------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FR-009-AC-1 | A manifest without a `name` falls back to the directory basename                                   | Test (catalog.test.ts)              |
| FR-009-AC-2 | Modules with and without `version`, and artifacts with and without a schema ref, resolve correctly | Test (catalog.test.ts)              |
| FR-009-AC-3 | Non-array and malformed type entries are ignored                                                   | Test (catalog.test.ts)              |
| FR-009-AC-4 | A skeleton resolves via the lowercase-filename fallback                                            | Test (write.test.ts, index.test.ts, catalog.test.ts) |
| FR-009-AC-5 | A skeleton named with the type's own casing resolves, and a filename in any other casing does not — on a case-insensitive filesystem as on a case-sensitive one | Test (catalog.test.ts)              |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-002](../usecase/US-002-discover-authoring-contract.md).
- **Downstream**: entries feed lookup ([FR-010](./FR-010-case-insensitive-type-lookup.md)),
  display ([FR-011](./FR-011-list-and-show-catalog.md)), and the authoring contract
  ([FR-014](./FR-014-emit-authoring-contract.md)).
