---
id: NFR-008
title: "Corrupt manifests abort assembly rather than drop silently"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-006"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-009"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
---

# NFR-008: Corrupt manifests abort assembly rather than drop silently

## Statement

A candidate module path that has no `manifest.yaml` SHALL be skipped during
catalog assembly, while a present-but-unparseable `manifest.yaml` SHALL abort
assembly, so that a corrupt installed module is surfaced rather than hidden behind
a partial catalog.

## Scope

- Applies to: catalog assembly when reading each candidate's manifest.
- Operational context: every catalog read over the module store.

## Rationale

A missing manifest legitimately means "not a module" and should be skipped. A
corrupt manifest, by contrast, means an installed module is broken; silently
dropping it would yield a partial catalog whose missing types confuse the author.
Aborting makes the corruption visible at the source.

## Measurement and Evaluation

| Metric                                                            | Target | Threshold | Method |
| ----------------------------------------------------------------- | ------ | --------- | ------ |
| Corrupt manifests that abort assembly (rather than drop silently) | 100%   | 100%      | Test   |
| Missing manifests that are skipped (rather than abort)            | 100%   | 100%      | Test   |

## Verification

The catalog tests assert that a candidate without a manifest is skipped and that a
present-but-unparseable manifest causes assembly to throw rather than return a
partial catalog (`catalog.test.ts`).

## Dependencies

- **Upstream**: [FR-006](../functional/FR-006-locate-module-roots.md) and
  [FR-009](../functional/FR-009-read-module-manifest.md), whose manifest handling
  this constrains.
- **Downstream**: none.
