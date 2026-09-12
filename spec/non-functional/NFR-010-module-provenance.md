---
id: NFR-010
title: "An installed module is the one that was pinned"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/quoin/FR-018"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# NFR-010: An installed module is the one that was pinned

## Statement

`quoin` SHALL resolve every default-set module to an **immutable** revision.
`quoin` SHALL record the resolved commit SHA alongside the human-readable pin, so
that a later install of the same pin either yields the same tree or reports that
it does not.

## Scope

- Applies to: `default-modules.yaml` pins and the install path that materializes
  them into `~/.ix/filament/modules`.
- Operational context: any `quoin module ensure-defaults` or
  `quoin plugin install`, including the lazy install `quire validate` triggers.
- Excluded: the transport itself. Fetching is `@agent-ix/ts-plugin-kit`'s
  responsibility (`git clone --filter=blob:none` + sparse checkout); this
  requirement is about **what quoin asks for and what it records**.

## Rationale

The default set is pinned by **git tag** — `ref: v0.15.0`. A tag is a mutable
pointer: repointing it changes what the same pin resolves to, and nothing today
would notice. Git verifies object integrity within a fetch, so this is not about
corruption in transit; it is about the pin denoting one tree over time.

The blast radius grew during ADR-0011 Phase 2. Module data is no longer only
document schemas: `traceability.vocabulary_coverage` and the
`verification_catalog` now decide **what quoin checks and what it will accept as
verified** (FR-031, FR-037). A module that silently changes underneath a pin
changes the verdicts a repository's CI produces.

Recording the resolved SHA is the smallest thing that makes the pin mean
something. It does not require signing, a registry, or a lockfile format — the
existing `release-drift pins` report is the natural place for the comparison.

## Measurement and Evaluation

| Metric                                                          | Target | Threshold | Method      |
| --------------------------------------------------------------- | ------ | --------- | ----------- |
| Default-set pins recording the commit SHA their ref resolved to | 100%   | 100%      | Test        |
| Installs that proceed when a recorded SHA no longer matches     | 0      | 0         | Test        |
| Module fetches from a source outside the declared pin set       | 0      | 0         | Inspection  |

## Verification

`default-modules.yaml` is inspected for a resolved-SHA field on every entry, and
the install path is exercised against a pin whose recorded SHA does not match the
tree it resolves to.

**Not met at the time of writing.** `default-modules.yaml` records `version` and
`ref` only. This requirement is stated ahead of the implementation, which is the
spec-first order; the gap is `agent-ix/quoin#132`.

## Dependencies

- **Upstream**: [StR-001](../stakeholder/StR-001-standalone-cli.md) standalone CLI.
- **Downstream**: the pin drift report (`scripts/release-drift.js pins`) is where
  a mismatch surfaces.
