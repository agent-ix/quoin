---
id: NFR-012
title: "Version skew across the ecosystem is reported, never absorbed"
type: NFR
quality_attribute: compatibility
relationships:
  - target: "ix://agent-ix/quoin/FR-029"
    type: "constrains"
  - target: "ix://agent-ix/quoin/NFR-007"
    type: "constrains"
---

# NFR-012: Version skew across the ecosystem is reported, never absorbed

## Statement

Where `quoin` depends on a capability of `quire` or of a spec module, it SHALL
state the version that capability arrived in, and SHALL report skew as skew — a
component too old to answer the question SHALL NOT present as a component
answering it in the negative.

## Scope

- Applies to: the `quire` CLI contract (`src/quire/contract.ts`), the module set
  pinned in `default-modules.yaml`, and the agent surfaces skills are emitted for.
- Operational context: any invocation that reads a module declaration or a quire
  output envelope.

## Rationale

quoin sits between components that release independently: an engine crate, a CLI
binary, a Python wheel, nine spec modules and three agent surfaces. **Skew is the
normal state, not the exception.**

Its characteristic failure is not a crash — it is a **clean result**. Both halves
of FR-037 demonstrated it on the same afternoon:

- the installed `quire` was **0.22.0** while the capability shipped in the CLI
  that pins engine v0.33.0, so the check ran and reported nothing;
- the module pin was **iso v0.14.0**, one release before the declaration the check
  reads, so there was nothing to walk.

Neither presents as a failure. Both present as a bundle with no findings, which is
indistinguishable from a bundle that is complete. That is the same shape as the
1,014 trace tags binding to nothing while matrices read as covered — the defect
class this whole program exists to catch, arriving through version skew instead of
through dead links.

`FR-029`'s version premise already does this for the quire contract, and
`release-drift pins` does it for module pins. This states the property both
implement, so a third integration point cannot be added without one.

## Measurement and Evaluation

| Metric                                                                     | Target | Threshold | Method     |
| -------------------------------------------------------------------------- | ------ | --------- | ---------- |
| Capabilities used without a declared minimum version                       | 0      | 0         | Inspection |
| Skew conditions reported as a diagnostic rather than an empty result       | 100%   | 100%      | Test       |
| Module pins behind their latest release at merge                           | 0      | 0         | Test       |

## Verification

`checkVersionPremise` is exercised against a `quire` below the declared floor and
asserted to name found, required and consequence (TC-114). `release-drift pins`
is run over the pin set. The source is inspected for reads of module data whose
introducing version is not declared.

## Dependencies

- **Upstream**: [FR-029](../functional/FR-029-consume-the-quire-json-contract.md)
  (the version premise), [NFR-007](./NFR-007-external-tool-invocation.md)
  (external tools resolved from `PATH`, unpinned by design).
- **Downstream**: [NFR-009](./NFR-009-single-source-skills-across-agents.md), whose
  agent surfaces are the third skewing axis.
