---
id: FR-025
title: "Resolve the authoring organization"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-010"
    type: "implements"
---

# FR-025: Resolve the authoring organization

## Description

The `quoin` CLI SHALL resolve the authoring organization for a repository from
the first available of an explicit `--org` value, the `QUOIN_ORG` environment
variable, and the `origin` remote recorded in the repository's Git configuration.

The `quoin` CLI SHALL report the authoring organization as unresolved when no
source yields one.

The `quoin` CLI SHALL NOT substitute a default authoring organization.

## Inputs

- The resolved repository root.
- The optional `--org <name>` flag for the invocation.
- The `QUOIN_ORG` environment variable.
- The `[remote "origin"]` URL in `<repo_root>/.git/config`, when readable.

## Outputs

- A resolved organization name and the source it came from (`flag`, `env`, or
  `git`), or an unresolved result carrying neither.

## Behavior

- The command SHALL apply the precedence `--org`, then `QUOIN_ORG`, then the
  `origin` remote, and SHALL stop at the first source that yields a non-empty
  organization.
- The command SHALL parse the organization from both the SSH remote form
  (`git@<host>:<org>/<repo>.git`) and the HTTPS remote form
  (`https://<host>/<org>/<repo>.git`).
- The command SHALL read `.git/config` directly, so resolution holds where no Git
  executable is present
  ([NFR-004](../non-functional/NFR-004-standalone-dependencies.md)).
- The command SHALL NOT invoke `git` to resolve the organization.
- The command SHALL treat a missing `.git/config`, a configuration with no
  `origin` remote, and a remote URL with no parsable `<org>/<repo>` tail as
  yielding no organization, and SHALL continue to the unresolved result rather
  than failing.
- The command SHALL report an unresolved organization together with the remedy of
  passing `--org`, and SHALL NOT emit a placeholder, a sentinel, or any other
  substituted value in its place
  ([NFR-003](../non-functional/NFR-003-actionable-errors.md)).
- The command SHALL report the resolved organization and its source in the
  authoring pack emitted by [FR-014](./FR-014-emit-authoring-contract.md), in both
  the text and `--json` renderings.

## Rationale

An organization qualifies a repository so that same-named repositories owned by
different organizations remain distinguishable, so substituting a default defeats
the purpose of carrying one. A declared organization is also human-facing
identity in a published artifact, where a plausible but wrong value is harder to
notice, and worse once noticed, than an absent one that stops the author and asks.

## Acceptance Criteria

| ID          | Criteria                                                                                       | Verification                    |
| ----------- | ---------------------------------------------------------------------------------------------- | ------------------------------- |
| FR-025-AC-1 | `--org` takes precedence over `QUOIN_ORG`, which takes precedence over the `origin` remote      | Test (org.test.ts)              |
| FR-025-AC-2 | The organization is parsed from an SSH remote URL                                               | Test (org.test.ts)              |
| FR-025-AC-3 | The organization is parsed from an HTTPS remote URL                                             | Test (org.test.ts)              |
| FR-025-AC-4 | A missing `.git/config`, an absent `origin` remote, and an unparsable remote each resolve to unresolved without failing | Test (org.test.ts)              |
| FR-025-AC-5 | An unresolved organization is reported with the `--org` remedy and no substituted value         | Test (org.test.ts, write.test.ts) |
| FR-025-AC-6 | The authoring pack carries the organization and its source in text and under `--json`           | Test (write.test.ts, cli.test.ts) |
| FR-025-AC-7 | Resolution succeeds with no `git` executable on `PATH`                                          | Test (org.test.ts)              |

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI;
  [US-010](../usecase/US-010-author-specs-for-my-own-organization.md). Reads the
  configuration surface defined by
  [FR-023](./FR-023-runtime-configuration.md).
- **Downstream**: the resolved organization is carried in the authoring pack
  emitted by [FR-014](./FR-014-emit-authoring-contract.md).
