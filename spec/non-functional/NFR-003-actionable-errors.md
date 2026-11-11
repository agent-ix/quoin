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

`quoin-cli` `tests/tc_shell_contract.rs` ::
`tc_1650_unknown_command_is_a_stderr_refusal_with_the_root_catalogue` and
`tc_1650_nested_unknown_command_keeps_the_oclif_path_identity` assert that unknown commands
and subcommands are refused with a non-zero exit. No native test is cited for unknown
types, missing manifests, or catalog conflicts (PLAT-1275).

NFR-003-M-1 and NFR-003-M-2 are measured over the commands the `quoin` binary lists, each
given an unknown flag, by plans
[MP-250](../assurance/MP-250-nfr003-actionable-failures.md) and
[MP-251](../assurance/MP-251-nfr003-silent-failures.md) with the test
`tc_3003_every_command_fails_a_bad_argument_out_loud` (FR-131-AC-9). That population
covers argument validation of top-level commands only.

## Dependencies

- **Upstream**: [FR-005](../functional/FR-005-reject-unknown-commands.md),
  [FR-012](../functional/FR-012-detect-duplicate-types.md), and
  [FR-013](../functional/FR-013-resolve-requested-types.md), whose failure modes
  this constrains.
- **Downstream**: none.
