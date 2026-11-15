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

The quoin CLI SHALL read the committed default module set from the
`ts-plugin-kit` marketplace manifest at `default-modules.yaml`. Each default
module SHALL declare a typed source named by repository. The quoin CLI SHALL
use a module's release tag as `source.ref` when its repository provides one.
The quoin CLI SHALL use the repository's default branch as `source.ref` when
its repository provides no release tag for that module. This document defines
the structural schema of that file; its
behavioral installation is specified by
[FR-017](./FR-017-reconcile-default-modules.md).

## Properties

| Field         | Type       | Multiplicity | Constraints    |
| ------------- | ---------- | ------------ | -------------- |
| schemaVersion | Integer    | 1..1         | min: 1, max: 1 |
| name          | String     | 0..1         |                |
| entries       | JsonObject | 0..*         |                |

The table declares the manifest's top-level fields and their cardinality. Which
keys are required, and the nested shape of each entry, are stated only by the
schema below, which is the full contract. `schemaVersion` is the constant `1`.

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
              "ref": { "type": "string" },
              "package": { "type": "string" }
            }
          }
        }
      }
    }
  }
}
```

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-016-AC-1 | `default-modules.yaml` validates as a `ts-plugin-kit` marketplace manifest. | Test (TC-014) |
| FR-016-AC-2 | The committed file declares exactly ten public default modules, each with a typed source, and excludes private opt-in modules. | Test (TC-014) |
| FR-016-AC-4 | Each Git source sets `source.ref` to a release tag when one exists for its module, and uses the repository's default branch only when no release tag exists. | Test (TC-014) |
| FR-016-AC-5 | A declared `source.ref` is a string. | Test (TC-014) |

> **CR note (2026-09-29, agent-ix/quoin#654):** FR-016-AC-3 is withdrawn. It
> required every committed copy of a module's pin to agree and the manifest's
> declared `version` to equal the crate dependency's. That is a check over
> recorded versions, not behaviour: the `version` field is removed from the
> manifest and from `MarketplaceEntry`, and its test, `tc_1032_pin_agreement.rs`,
> is deleted. The `version` property is removed from the schema above.
> TC-1929 is withdrawn with it. The id is not reused.

## Dependencies

- **Upstream**: [StR-005](../stakeholder/StR-005-offline-reproducible.md)
  offline-safe, reproducible authoring;
  [US-001](../usecase/US-001-author-root-artifact-with-objects.md).
- **Downstream**: [FR-017](./FR-017-reconcile-default-modules.md) installs the
  modules this schema describes.
