---
id: FR-051
title: "Snapshot the semantic audit scope and provenance"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-014"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "references"
---

# FR-051: Snapshot the semantic audit scope and provenance

> **⛔ Withdrawn — 2026-09-29.** This requirement is withdrawn and is not
> implemented, with the rest of the semantic type-fit audit (US-014, FR-051..FR-055,
> NFR-015, NFR-016) under [quoin#654](https://github.com/agent-ix/quoin/pull/654).
> The audit ran once under [quoin#288](https://github.com/agent-ix/quoin/issues/288);
> its generator and test were removed by the native cutover
> ([quoin#524](https://github.com/agent-ix/quoin/pull/524)), and its retained output
> is deleted by quoin#654. Nothing in quoin produces or reads the audit any more.
>
> The text below is retained unchanged. It describes what was required, not what
> is required.

## Description

When the default-module semantic audit runs, it SHALL identify every input strongly enough to
reproduce the census and expose any disagreement among declared, resolved, installed, and reviewed
revisions.

## Rationale

A mutable checkout name or package version is not enough evidence for an ecosystem-wide design
review. The audit must distinguish what Quoin requested from the exact bytes it inspected.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-051-AC-1 | The snapshot records the audit timestamp, Quoin repository commit and cleanliness, `default-modules.yaml` content digest, and Quoin package version. | Test (TC-1156) |
| FR-051-AC-2 | Every `default-modules.yaml` entry records its name, source kind, canonical repository and subdirectory, requested version/ref, and resolved full commit SHA. | Test (TC-1157) |
| FR-051-AC-3 | Every inspected module records its content digest, manifest-declared name/version, source path, source commit when available, and checkout cleanliness when applicable. | Test (TC-1158) |
| FR-051-AC-4 | The snapshot records the Quire CLI and engine identities, the pinned `quire-rs#385` corpus revision, and the `filament-core-data#10` census revision. | Test (TC-1159) |
| FR-051-AC-5 | A disagreement among requested ref, resolved SHA, installed content, manifest identity, or canonical source is retained as a typed `provenance-conflict` with both values and blocks a clean audit verdict. | Test (TC-1160) |
| FR-051-AC-6 | Entries are ordered by their declaration order and identities use repository-relative POSIX paths and lowercase hexadecimal digests so equal inputs serialize byte-identically. | Test (TC-1161) |

## Constraints

- Resolving provenance may read local repositories and remote metadata but SHALL NOT change a source
  checkout, install modules, or rewrite the registry.

## Dependencies

- **Upstream**: [US-014](../usecase/US-014-audit-default-module-semantic-fit.md)
- **External**: `default-modules.yaml`, `agent-ix/quire-rs#385`, `agent-ix/filament-core-data#10`
