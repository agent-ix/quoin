---
id: NFR-007
title: "External tools are invoked by name and surface their failures"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-015"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-022"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# NFR-007: External tools are invoked by name and surface their failures

## Statement

The external tools `ix-flow` and `quire` SHALL be invoked by name from `PATH`,
their unavailability or non-zero exit SHALL surface as a non-zero `quoin` exit,
and the `quire` validation command SHALL be emitted for the calling agent to run
rather than executed by `quoin`. The external tools are not version-pinned, which
is a known and accepted limitation.

## Scope

- Applies to: the workflow launch path, the self-update path, and the emitted
  validation command.
- Operational context: any invocation that reaches an external tool.

## Rationale

Resolving external tools from `PATH` keeps `quoin` decoupled from their release
cycles and out of its dependency graph. Surfacing their failures as a non-zero
exit makes problems visible to the caller, and emitting (rather than running) the
`quire` command lets the agent own validation in its own working directory.

## Measurement and Evaluation

| Metric                                                          | Target | Threshold | Method |
| --------------------------------------------------------------- | ------ | --------- | ------ |
| External-tool failures surfaced as a non-zero `quoin` exit      | 100%   | 100%      | Test   |
| `quire` validation runs executed by `quoin` rather than emitted | 0      | 0         | Test   |

## Verification

The flow tests assert that a missing `ix-flow` and a non-zero `ix-flow` exit both
surface as a non-zero exit (`flows.test.ts`), and the write tests assert the
`quire` command is emitted, not executed (`write.test.ts`).

## Acceptance Criteria


| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-007-AC-1 | The absence of external-tool version pinning is an accepted limitation, recorded here rather than as a metric; it is revisited if pinning becomes required. | Demonstration |

## Dependencies

- **Upstream**: [FR-015](../functional/FR-015-emit-quire-validate-command.md),
  [FR-021](../functional/FR-021-launch-ix-flow-runs.md), and
  [FR-022](../functional/FR-022-self-update.md), whose external-tool invocation
  this constrains.
- **Downstream**: none.
