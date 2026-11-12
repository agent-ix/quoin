---
id: NFR-004
title: "quoin runs as a standalone package"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# NFR-004: quoin runs as a standalone package

## Statement

`quoin` SHALL run standalone: its only runtime dependencies SHALL be
`@agent-ix/ix-cli-core`, `@agent-ix/ts-plugin-kit`, and a YAML parser, while
`ix-flow` SHALL be invoked as an external process rather than linked. `quire` is
linked as the library `quire-rs`.

## Scope

- Applies to: the package's declared runtime dependency set.
- Operational context: installation and execution outside the `ix` umbrella.

## Rationale

A small, fixed runtime dependency set is what lets `quoin` install and run on its
own in any repository. Invoking `ix-flow` as a process keeps it out of the
dependency graph, so the install footprint stays minimal and the tool stays
usable independently of the platform.

## Measurement and Evaluation

| Metric                                                                    | Target | Threshold | Method     |
| ------------------------------------------------------------------------- | ------ | --------- | ---------- |
| Runtime dependencies beyond ix-cli-core, ts-plugin-kit, and a YAML parser | 0      | 0         | Inspection |
| `ix-flow` linked rather than invoked as a process                         | 0      | 0         | Inspection |

## Verification

The package's runtime dependencies are inspected against `package.json`, and the
source is inspected to confirm `ix-flow` is spawned, not imported.

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: process invocation of `ix-flow` is governed by
  [NFR-007](./NFR-007-external-tool-invocation.md).
