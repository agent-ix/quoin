---
id: NFR-003
title: "Failures surface as actionable errors"
type: NFR
quality_attribute: usability
relationships:
  - target: "ix://agent-ix/quoin/FR-005"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-012"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-013"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# NFR-003: Failures surface as actionable errors

## Statement

Command, argument, and lookup failures SHALL surface as raised errors or non-zero
exit codes carrying actionable messages — available types, usage text, or source
diagnostics — so that a caller can correct the input rather than face silent or
partial output.

## Scope

- Applies to: command dispatch, argument validation, and catalog/type lookup.
- Operational context: any invalid or unresolved invocation.

## Rationale

The primary caller is an agent that must self-correct. An actionable message — the
sorted list of available types, the relevant usage, or a named conflict — lets the
agent recover in one step, whereas a silent failure stalls the loop.

## Measurement and Evaluation

| Metric                                                         | Target | Threshold | Method |
| -------------------------------------------------------------- | ------ | --------- | ------ |
| Failure paths that emit an actionable message or non-zero exit | 100%   | 100%      | Test   |
| Silent or partial-output failures                              | 0      | 0         | Test   |

## Verification

Error-path tests assert that unknown commands and subcommands, unknown types
(with the available-type list), missing manifests, and catalog conflicts each
raise an actionable error or set a non-zero exit (`cli.test.ts`,
`write.test.ts`, `plugins.test.ts`).

## Dependencies

- **Upstream**: [FR-005](../functional/FR-005-reject-unknown-commands.md),
  [FR-012](../functional/FR-012-detect-duplicate-types.md), and
  [FR-013](../functional/FR-013-resolve-requested-types.md), whose failure modes
  this constrains.
- **Downstream**: none.
