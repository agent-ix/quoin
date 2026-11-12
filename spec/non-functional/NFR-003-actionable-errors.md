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
and subcommands are refused with a non-zero exit.

NFR-003-M-1 and NFR-003-M-2 are measured over command x error-class pairs by plans
[MP-250](../assurance/MP-250-nfr003-actionable-failures.md) and
[MP-251](../assurance/MP-251-nfr003-silent-failures.md), whose declared set the test
`tc_3003_the_nfr003_populations_are_stated_by_class` (FR-131-AC-9) ties to what the producer
enumerates from the binary's help text. The classes are unknown flag (TC-3700), unknown
subcommand (TC-3701), missing argument (TC-3702), missing path (TC-3703), unknown type
(TC-3704), malformed JSON input (TC-3705) and catalog conflict (TC-3706); each is a module
under `tests/nfr_populations/` and its test names the pairs that read below the target. A
pair is actionable when the message names the cause and points to a next step. The population
is the failures the help text lets the producer construct: it excludes commands with a second
required argument for the path and input classes, failures that need a staged evidence store
or vocabulary module, and failures inside a running command.

## Dependencies

- **Upstream**: [FR-005](../functional/FR-005-reject-unknown-commands.md),
  [FR-012](../functional/FR-012-detect-duplicate-types.md), and
  [FR-013](../functional/FR-013-resolve-requested-types.md), whose failure modes
  this constrains.
- **Downstream**: none.
