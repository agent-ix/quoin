---
id: FR-023
title: "quoin runtime configuration surface"
type: FR
object: configuration
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-004"
    type: "specifies"
  - target: "ix://agent-ix/quoin/FR-007"
    type: "specifies"
  - target: "ix://agent-ix/quoin/FR-025"
    type: "specifies"
---

# FR-023: quoin runtime configuration surface

## Description

`quoin` SHALL read its configuration from a defined surface of environment
variables and command-line flags that select the config root, the module search
paths, the workflow skill root, the authoring organization, and per-invocation
output and registry options. This document defines that configuration surface;
the behavior that consumes it is specified by
[FR-004](./FR-004-resolve-config-root.md),
[FR-007](./FR-007-assemble-module-roots.md), and
[FR-025](./FR-025-resolve-authoring-organization.md).

## Configuration

| Name                   | Scope   | Type    | Default              | Description                                                                            |
| ---------------------- | ------- | ------- | -------------------- | -------------------------------------------------------------------------------------- |
| IX_HOME                | runtime | string  | ~/.ix                | Config root and parent of the shared module store, used when `--config-root` is absent |
| QUOIN_MODULE_PATHS     | runtime | string  | (unset)              | Colon-separated module roots assembled before the installed store                      |
| IX_SPEC_WORKFLOWS_ROOT | runtime | string  | (unset)              | First candidate root searched when resolving a workflow skill                          |
| QUOIN_ORG              | runtime | string  | (unset)              | Authoring organization, used when `--org` is absent and before the `origin` remote     |
| --config-root          | session | string  | IX_HOME or ~/.ix     | Config root for a single invocation                                                    |
| --org                  | session | string  | (unset)              | Authoring organization for a single invocation, ahead of every other source            |
| config.d/quoin.yaml    | stored  | file    | (absent)             | Persisted quoin configuration; `org` is read behind `--org`/`QUOIN_ORG` and ahead of the git remote |
| --no-project-config    | session | boolean | false                | Disables the `<cwd>/.ix` project config root for the invocation                        |
| --json                 | session | boolean | false                | Renders command output as JSON                                                         |
| --registry             | session | string  | (ambient npm config) | Registry used by `update` to query and install                                         |

## Behavior

The CLI reads each environment variable on every invocation, so a change takes
effect on the next run. A `--config-root` flag overrides `IX_HOME` for that
invocation; the resolved root is exported to `IX_HOME` so downstream resolution
is consistent. Flag values apply only to the invocation that carries them.

The stored file is layered by `ConfigService`: a project-local `.ix` config
overrides the user-level one, and a bound environment variable overrides both
([FR-027](./FR-027-store-the-authoring-organization.md)).

Every entry above except the authoring organization has a defined default that
applies when the entry is absent. The organization has none: when neither `--org`
nor `QUOIN_ORG` is set and no organization can be read from the repository, it
stays unresolved rather than falling back to a value
([FR-025](./FR-025-resolve-authoring-organization.md)).

## Acceptance Criteria

| ID          | Criteria                                                                                | Verification                        |
| ----------- | --------------------------------------------------------------------------------------- | ----------------------------------- |
| FR-023-AC-1 | `--config-root`, then `IX_HOME`, then `~/.ix` select the config root in that precedence | Test (cli.test.ts, catalog.test.ts) |
| FR-023-AC-2 | `QUOIN_MODULE_PATHS` roots are assembled before the installed store                     | Test (catalog.test.ts)              |
| FR-023-AC-3 | `IX_SPEC_WORKFLOWS_ROOT` is the first candidate when resolving a workflow skill         | Test (flows.test.ts)                |
| FR-023-AC-4 | `--org`, then `QUOIN_ORG` select the authoring organization in that precedence, and neither defaults when absent | Test (org.test.ts, cli.test.ts) |
| FR-023-AC-5 | The stored config file supplies the organization behind `--org`/`QUOIN_ORG` and ahead of the git remote | Test (org.test.ts) |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: consumed by [FR-004](./FR-004-resolve-config-root.md),
  [FR-007](./FR-007-assemble-module-roots.md),
  [FR-020](./FR-020-resolve-workflow-skills.md),
  [FR-025](./FR-025-resolve-authoring-organization.md), and
  [FR-027](./FR-027-store-the-authoring-organization.md).
