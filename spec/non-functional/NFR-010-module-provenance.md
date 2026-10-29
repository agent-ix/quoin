---
id: NFR-010
title: "A default module is declared at an immutable revision"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/quoin/FR-018"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# NFR-010: A default module is declared at an immutable revision

## Statement

`quoin` SHALL resolve every default-set module from a `ref` that is a full git
commit id, which names one tree for ever. `quoin` SHALL refuse a `ref` that is a
branch or tag name for a default-set entry, and SHALL record no second copy of
the resolved commit and compare nothing against it: the declared `ref` is the
whole truth.

## Scope

- Applies to: `default-modules.yaml` entries and the install path that
  materializes them into `~/.ix/filament/modules`.
- Operational context: any `quoin module ensure-defaults` or
  `quoin plugin install`, including the lazy install `quire validate` triggers.
- Excluded: the transport itself, which is `@agent-ix/ts-plugin-kit`'s
  responsibility.

## Rationale

A tag is a mutable pointer: repointing it changes what the same declaration
resolves to. A commit id cannot be repointed, so declaring one makes the
declaration denote one tree without a recorded SHA, a lockfile, or a
comparison step. Module data decides what quoin checks and accepts as verified
(FR-031, FR-037), so the declaration must not float.

## Measurement and Evaluation

| Metric                                                      | Target | Threshold | Method     |
| ----------------------------------------------------------- | ------ | --------- | ---------- |
| Default-set entries whose `ref` is a full commit id         | 100%   | 100%      | Test       |
| Installs that proceed for a branch or tag `ref`             | 0      | 0         | Test       |
| Module fetches from a source outside the declared entries   | 0      | 0         | Inspection |

## Verification

`default-modules.yaml` is inspected so every entry's `ref` is a full commit id,
and the install path is exercised against a tag `ref`, which it refuses.

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: none.
