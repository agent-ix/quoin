---
id: FR-014
title: "Emit an authoring contract per resolved type"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-001"
    type: "implements"
---

# FR-014: Emit an authoring contract per resolved type

## Description

For each resolved type, the `write` command SHALL emit an authoring contract —
the type name, kind, module name, module root, and any skeleton and schema paths
— rendered as text or as JSON under `--json`, and SHALL mark a type that has
neither a skeleton nor a schema as "manifest only".

## Inputs

- The resolved catalog entries and the optional `--json` flag.

## Outputs

- An authoring pack listing one contract per type, plus the repo root and the
  validation command, as text or JSON.

## Behavior

- The command SHALL render each type's name, kind, module name, and module root.
- The command SHALL render the skeleton path and schema path when present.
- The command SHALL mark a type with neither skeleton nor schema as
  "manifest only".
- The command SHALL render the whole pack as JSON when `--json` is given.

## Acceptance Criteria

| ID          | Criteria                                                          | Verification                      |
| ----------- | ----------------------------------------------------------------- | --------------------------------- |
| FR-014-AC-1 | Each type renders name, kind, module, and root as text            | Test (write.test.ts, cli.test.ts) |
| FR-014-AC-2 | Skeleton and schema paths render when present                     | Test (write.test.ts)              |
| FR-014-AC-3 | A type with neither skeleton nor schema is marked "manifest only" | Test (write.test.ts)              |
| FR-014-AC-4 | `--json` renders the whole pack as JSON                           | Test (write.test.ts, cli.test.ts) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-001](../usecase/US-001-author-root-artifact-with-objects.md). Consumes
  [FR-013](./FR-013-resolve-requested-types.md).
- **Downstream**: the pack also carries the validation command from
  [FR-015](./FR-015-emit-quire-validate-command.md) and the authoring
  organization from [FR-025](./FR-025-resolve-authoring-organization.md).
