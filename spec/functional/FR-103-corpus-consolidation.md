---
id: FR-103
title: "Consolidate the Quoin side of corpus tooling"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
---

# FR-103: Consolidate the Quoin side of corpus tooling

## Description

Quoin SHALL consolidate the corpus accounting and selection behaviour it owns
into one Rust implementation obtained from `engineering-assurance`, SHALL treat
the `corpus/` submodule's contents as another repository's tree that Quoin may
read but not retire, and SHALL change no accepted-corpus byte.

## Inputs

- The `corpus/` git submodule, pointing at `agent-ix/qa-corpus` at its pinned
  revision.
- The governed corpus pins consumed by the measurement crates.
- The Quoin-side corpus selection and accounting code reached from `src/` and
  `scripts/`.
- `engineering-assurance`'s published corpus-accounting capability.

## Outputs

- A corpus-accounting record naming the corpus identity, its pinned submodule
  revision and the population it covers.
- A final disposition for each Quoin-side corpus module: reimplement in Rust,
  retain as approved code, or delete.
- A recorded statement of which corpus concerns are out of Quoin's ownership,
  pending the owner scope ruling.

## Behavior

- Quoin SHALL scope this consolidation to the corpus accounting and selection its
  own measurement crates consume.
- Quoin SHALL NOT record a port or delete disposition for a file inside the
  `corpus/` submodule; an inert allowance entry is permitted, because it records
  that the file determines no assertion rather than an intent to change it.
- Quoin SHALL record that corpus construction and reproduction from recorded
  sources belong to the repository the submodule points at, and SHALL treat the
  question of whether this programme has any scope over `agent-ix/qa-corpus` as
  an open owner ruling rather than assuming either answer.
- Quoin SHALL pin the submodule to an exact revision and SHALL report a
  submodule revision change as a corpus identity change rather than absorbing it.
- Quoin SHALL obtain corpus accounting from `engineering-assurance` rather than
  implementing a local equivalent, and if `engineering-assurance` does not supply
  it, then Quoin SHALL file a gap ticket in `engineering-assurance`.
- Quoin SHALL record one disposition per Quoin-side corpus module and SHALL leave
  no such module unclassified.
- Quoin SHALL classify a foreign-language sample input that determines no
  assertion as inert data through the allowance manifest, identified by its
  declared allowance rather than by its directory name, and SHALL exclude it from
  executable debt.
- Quoin SHALL complete this consolidation before the measurement crates are
  ported, so that the measurement port reads one corpus source.
- Quoin SHALL verify corpus identity by digest before and after the
  consolidation and SHALL change no accepted-corpus byte.
- Quoin SHALL publish no corpus tooling package to public npmjs or crates.io, and
  SHALL publish any retained Python package to the internal PyPI.

## Error Conditions

A corpus digest that differs before and after consolidation, an unclassified
Quoin-side corpus module, a measurement run that resolves two corpus sources, an
unpinned or missing submodule revision, and a port or delete disposition
recorded against a file inside the submodule each block consolidation.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-103-CON-1 | Consolidation SHALL NOT change an accepted-corpus byte. | Data Integrity | Test |
| FR-103-CON-2 | Quoin SHALL NOT record a port or delete disposition for a file inside the `corpus/` submodule. | Scope | Test |
| FR-103-CON-3 | Quoin SHALL NOT open a corpus debt row that the containment matrix does not carry. | Scope | Inspection |
| FR-103-CON-4 | Inert sample inputs SHALL NOT be counted as executable debt. | Reporting | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-103-AC-1 | Every Quoin-side corpus module carries one recorded disposition, and the gate fails when one is unclassified. | Test (TC-1655) |
| FR-103-AC-2 | The accepted-corpus digest computed before consolidation equals the digest computed after it, over a named non-empty population. | Test (TC-1656) |
| FR-103-AC-3 | A measurement run resolves exactly one corpus source, and a planted second source is refused. | Test (TC-1657) |
| FR-103-AC-4 | Corpus accounting is obtained from `engineering-assurance`, and no local corpus-accounting implementation exists in the workspace. | Test (TC-1658) |
| FR-103-AC-5 | A sample input declared inert in the allowance manifest is reported as an allowance, and the same file moved to an undeclared executable path is reported as a violation. | Test (TC-1659) |
| FR-103-AC-6 | A port or delete disposition recorded against a path inside the `corpus/` submodule fails the gate, naming the submodule and its upstream repository; an inert allowance entry covering the same path does not, because it records no intent to change that repository. | Test (TC-1684) |
| FR-103-AC-7 | The submodule is pinned to an exact revision, and a revision change is reported as a corpus identity change with both revisions named. | Test (TC-1685) |

## Dependencies

- **Upstream**: [FR-096](./FR-096-versioned-rust-engine-boundary.md); [NFR-024](../non-functional/NFR-024-bounded-staged-coexistence.md), which defines the allowance manifest; the owner ruling that `agent-ix/qa-corpus` is out of this programme's scope (2026-09-12), recorded in [ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md). This requirement runs in parallel with the delivery stages and is not sequenced behind [FR-101](./FR-101-retire-replaced-executable-paths.md).
- **Downstream**: none. This previously named [FR-100](./FR-100-rust-evidence-measurement-change-assurance.md) as consuming the consolidated accounting, which stopped being true when [#388](https://github.com/agent-ix/quoin/issues/388) disposed of the corpus-measurement subsystem: FR-100's measurement crates now carry the measurement-record capability and are required NOT to restate corpus accounting.
