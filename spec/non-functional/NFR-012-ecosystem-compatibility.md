---
id: NFR-012
title: "Version skew across the ecosystem is reported, never absorbed"
type: NFR
quality_attribute: compatibility
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-029"
    type: "constrains"
  - target: "ix://agent-ix/quoin/NFR-007"
    type: "constrains"
---

# NFR-012: Version skew across the ecosystem is reported, never absorbed

## Statement

Where `quoin` depends on a capability of `quire` or of a spec module, it SHALL
report skew as skew — a component too old to answer the question SHALL NOT
present as a component answering it in the negative. Quoin records no minimum
version and compares none.

## Scope

- Applies to: the `quire` CLI contract, the module set
  declared in `default-modules.yaml`, and the agent surfaces skills are emitted for.
- Operational context: any invocation that reads a module declaration or a quire
  output envelope.

## Rationale

quoin sits between components that release independently: an engine crate, a CLI
binary, a Python wheel, nine spec modules and three agent surfaces. **Skew is the
normal state, not the exception.**

Its characteristic failure is not a crash — it is a **clean result**. Both halves
of FR-037 demonstrated it on the same afternoon:

- the installed `quire` predated the CLI that carried the capability, so the check ran
  and reported nothing;
- the installed module predated the declaration the check
  reads, so there was nothing to walk.

Neither presents as a failure. Both present as a bundle with no findings, which is
indistinguishable from a bundle that is complete. That is the same shape as the
1,014 trace tags binding to nothing while matrices read as covered — the defect
class this whole program exists to catch, arriving through version skew instead of
through dead links.

FR-029 consumes the quire contract in process. This states the property, so a
second integration point cannot be added without it.

## Measurement and Evaluation

| Metric                                                                     | Target | Threshold | Method     |
| -------------------------------------------------------------------------- | ------ | --------- | ---------- |
| Skew conditions reported as a diagnostic rather than an empty result       | 100%   | 100%      | Test       |

## Verification

A component that cannot answer is exercised and asserted to produce a named
diagnostic rather than an empty result.

## Dependencies

- **Upstream**: [FR-029](../functional/FR-029-consume-the-quire-json-contract.md)
  (the quire JSON contract), [NFR-007](./NFR-007-external-tool-invocation.md)
  (external tools resolved from `PATH`).
- **Downstream**: [NFR-009](./NFR-009-single-source-skills-across-agents.md), whose
  agent surfaces are the third skewing axis.
