---
id: FR-016
title: "Default module set manifest schema"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quoin/StR-005"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-001"
    type: "implements"
---

# FR-016: Default module set manifest schema

## Description

The committed default module set SHALL be expressed as a `ts-plugin-kit`
marketplace manifest stored at `default-modules.yaml`, declaring each default
module with a typed source pinned to the release tag its package is built from
where a release tag exists; a module without a release tag MAY pin to its default
branch. The authoritative shape is `ts-plugin-kit`'s `validateMarketplaceManifest`;
the schema below documents that contract and is kept in step with it. This
document defines the structural schema of that file; its behavioral installation
is specified by
[FR-017](./FR-017-reconcile-default-modules.md).

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "DefaultModulesManifest",
  "type": "object",
  "required": ["schemaVersion", "entries"],
  "properties": {
    "schemaVersion": { "const": 1 },
    "name": { "type": "string" },
    "entries": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "source"],
        "properties": {
          "name": { "type": "string" },
          "version": { "type": "string" },
          "defaultEnabled": { "type": "boolean" },
          "source": {
            "type": "object",
            "required": ["type"],
            "properties": {
              "type": {
                "type": "string",
                "enum": ["git-subdir", "github", "path", "npm"]
              },
              "url": { "type": "string" },
              "repo": { "type": "string" },
              "path": { "type": "string" },
              "package": { "type": "string" },
              "ref": { "type": "string" }
            }
          }
        }
      }
    }
  }
}
```

## Acceptance Criteria

| ID          | Criteria                                                                               | Verification         |
| ----------- | -------------------------------------------------------------------------------------- | -------------------- |
| FR-016-AC-1 | `default-modules.yaml` validates as a `ts-plugin-kit` marketplace manifest             | Test (index.test.ts) |
| FR-016-AC-2 | The committed file declares the ten default modules, each with a typed pinned source | Test (index.test.ts) |

## Dependencies

- **Upstream**: [StR-005](../stakeholder/StR-005-offline-reproducible.md)
  offline-safe, reproducible authoring;
  [US-001](../usecase/US-001-author-root-artifact-with-objects.md).
- **Downstream**: [FR-017](./FR-017-reconcile-default-modules.md) installs the
  modules this schema describes.
