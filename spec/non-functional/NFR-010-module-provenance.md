---
id: NFR-010
title: "A default module is the one its declaration names"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/quoin/FR-018"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# NFR-010: A default module is the one its declaration names

## Statement

`quoin` SHALL resolve every default-set module from the `ref` its
`default-modules.yaml` declaration names, which may be a release tag or a
branch name, and SHALL refuse no `ref` for being a tag or a branch. `quoin`
SHALL NOT require a `ref` to be a full commit id, and SHALL record no resolved
commit and compare nothing against one: the declared `ref` is the whole truth.

## Scope

- Applies to: `default-modules.yaml` entries and the install path that
  materializes them into `~/.ix/filament/modules`.
- Operational context: any `quoin module ensure-defaults` or
  `quoin plugin install`, including the lazy install `quire validate` triggers.
- Excluded: the transport itself, which is `@agent-ix/ts-plugin-kit`'s
  responsibility.

## Rationale

A module declaration is data the module owner maintains, and the owner names
the revision by tag or branch. Quoin installs what the declaration names and
keeps no second record of the resolved revision, so there is no recorded
revision to drift from and no pinned commit to refresh.

## Measurement and Evaluation

| Metric                                                        | Target | Threshold | Method     |
| ------------------------------------------------------------- | ------ | --------- | ---------- |
| Default-set entries whose `ref` is a tag or branch name       | 100%   | 100%      | Inspection |
| Installs refused because a `ref` is a tag or a branch         | 0      | 0         | Test       |
| Module fetches from a source outside the declared entries     | 0      | 0         | Inspection |

## Verification

`default-modules.yaml` is inspected so every entry's `ref` is a name the module
repository declares and no entry carries a full commit id, and the install path
is exercised against a tag `ref`, which it installs.

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: none.
