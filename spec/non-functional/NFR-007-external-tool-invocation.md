---
id: NFR-007
title: "ix-flow is invoked by name and its failures surface"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-021"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# NFR-007: ix-flow is invoked by name and its failures surface

## Statement

`quoin` SHALL invoke the external tool `ix-flow` by name from `PATH`, and its
absence or non-zero exit SHALL surface as a non-zero `quoin` exit. `quire` is a
library linked into `quoin`, not a process; the only processes `quoin` starts are
`git`, `ix-flow` and the operator's `$EDITOR` (FR-036-AC-8). `ix-flow` is not
version-pinned, which is a known and accepted limitation.

## Scope

- Applies to: the workflow launch path.
- Operational context: any invocation that reaches `ix-flow`.

## Rationale

Resolving `ix-flow` from `PATH` keeps `quoin` decoupled from its release cycle and
out of its dependency graph. Surfacing its failures as a non-zero exit makes
problems visible to the caller.

## Measurement and Evaluation

| Metric                                                          | Target | Threshold | Method |
| --------------------------------------------------------------- | ------ | --------- | ------ |
| `ix-flow` failures surfaced as a non-zero `quoin` exit          | 100%   | 100%      | Test   |

## Verification

`quoin-cli` `src/flow.rs` ::
`ix_flow_error_envelope_is_reported_before_missing_run_id` asserts that an `ix-flow` failure is
reported. `tc_203` (FR-036-AC-8) asserts structurally that the executables `quoin` spawns are
exactly `git`, `ix-flow` and `$EDITOR`.

NFR-007-M-1 is measured over each workflow command with `ix-flow` absent and with `ix-flow`
failing, by plan [MP-252](../assurance/MP-252-nfr007-external-tool-failures.md) with the
test `tc_3004_an_ix_flow_failure_is_a_non_zero_quoin_exit` (FR-131-AC-10).

## Acceptance Criteria


| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-007-AC-1 | The absence of `ix-flow` version pinning is an accepted limitation, recorded here rather than as a metric; it is revisited if pinning becomes required. | Demonstration |

## Dependencies

- **Upstream**: [FR-021](../functional/FR-021-launch-ix-flow-runs.md), whose
  `ix-flow` invocation this constrains.
- **Downstream**: none.
