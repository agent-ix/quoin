---
id: FR-027
title: "Store the authoring organization"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-010"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-025"
    type: "specifies"
---

# FR-027: Store the authoring organization

## Description

`quoin` SHALL persist the authoring organization, so an author or an agent
states it once rather than on every invocation.

`quoin` SHALL read the stored organization ahead of the repository's `origin`
remote and behind `--org` and `QUOIN_ORG`.

`quoin` SHALL NOT treat a stored organization as a default: it is a value
someone recorded, where the outcome
[FR-025](./FR-025-resolve-authoring-organization.md) forbids is one quoin
invents.

## Inputs

- The stored configuration for plugin id `quoin`, layered per
  [FR-023](./FR-023-runtime-configuration.md).
- A key path and value, for the storing commands.

## Outputs

- A resolved organization reported with source `config`, or the stored file
  written in place.

## Behavior

- The command SHALL store configuration under the plugin id `quoin`, so a value
  written by `quoin config set` and one written by a host's `config set` reach
  the same file.
- The command SHALL declare a strict schema, so an unrecognized key is rejected
  rather than written.
- The command SHALL declare `QUOIN_ORG` as the environment binding for `org`,
  so the variable layers over the stored value rather than being read
  separately.
- The command SHALL layer a project-local configuration over the user-level one
  where the invocation enables it, so an organization can differ per
  repository without changing global state.
- Where the stored configuration cannot be parsed or does not validate, the
  command SHALL continue with no stored organization rather than failing, and
  the problem SHALL remain reportable
  ([NFR-003](../non-functional/NFR-003-actionable-errors.md)).
- `quoin` SHALL expose its configuration schema as the `ixSchema` named export
  of its package main, so a host that loads quoin registers the same schema
  quoin uses ([ix-cli-core FR-014](ix://agent-ix/ix-cli-core/FR-014)).
- The storing, reading, editing, and diagnostic commands SHALL delegate to the
  shared handlers in `@agent-ix/ix-cli-core` rather than reimplementing them.

## Rationale

`--org` lasts one invocation and `QUOIN_ORG` one shell, so neither answers "set
it once". A stored value does, and it outranks the remote because the remote is
only what quoin could infer while the stored value is what someone stated. That
ordering does not weaken FR-025: the rule there is that quoin must not invent an
organization, and a configuration file is the opposite of an invention.

## Acceptance Criteria

| ID          | Criteria                                                                                          | Verification                        |
| ----------- | --------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FR-027-AC-1 | A stored organization is used ahead of the `origin` remote and reported with source `config`        | Test (org.test.ts)                  |
| FR-027-AC-2 | `--org` takes precedence over a stored organization                                                 | Test (org.test.ts)                  |
| FR-027-AC-3 | `QUOIN_ORG` layers over a stored organization and is reported with source `env`                     | Test (org.test.ts)                  |
| FR-027-AC-4 | With nothing stored, resolution falls through to the remote, and to unresolved when there is none   | Test (org.test.ts)                  |
| FR-027-AC-5 | A malformed stored configuration leaves resolution to continue rather than failing the command      | Test (org.test.ts)                  |
| FR-027-AC-6 | The declared schema is strict, so storing an unrecognized key is rejected                           | Test (config-schema.test.ts)        |
| FR-027-AC-7 | `ixSchema` is exported from the package main, carrying the plugin id, schema, and env binding       | Test (config-schema.test.ts)        |
| FR-027-AC-8 | `config get`, `set`, `edit`, and `doctor` delegate to the shared ix-cli-core handlers               | Test (config-schema.test.ts, cli.test.ts) |
| FR-027-AC-9 | A project-local configuration overrides the user-level one, and is ignored where the invocation disables it | Test (org.test.ts)                  |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md);
  [US-010](../usecase/US-010-author-specs-for-my-own-organization.md).
  Consumes the configuration surface of
  [FR-023](./FR-023-runtime-configuration.md) and the `ixSchema` convention of
  [ix-cli-core FR-014](ix://agent-ix/ix-cli-core/FR-014).
- **Downstream**: supplies the `config` layer that
  [FR-025](./FR-025-resolve-authoring-organization.md) resolves through.
