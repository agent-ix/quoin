---
id: FR-101
title: "Lock semantic module, source and generated-package revisions"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-023"
    type: "traces_to"
---

# FR-101: Lock semantic module, source and generated-package revisions

## Description

Catalog resolution SHALL record, for every semantic module it resolves, the exact revisions the
resolution used, and SHALL resolve to the same revisions from a clean environment.

## Rationale

A catalog that resolves a module by a moving reference answers a different question on every run,
and the answer that matters — which contract revision produced a given artifact — becomes
unrecoverable after the fact. The campaign has already met that shape twice: a verification stack
whose `quire-rs` lock went stale reported nothing at all, and a module set the engine could not be
restricted to left every published rate unattributable.

Recording the revision is what makes a later measurement mean something. A lock that omits the
compiler version cannot distinguish two artifacts that differ only by the compiler that produced
them, and the whole point of pinning is to be able to make that distinction.

## Inputs

- A resolved `SpecCatalog`, carrying each module's root, name and parsed `semantic` block.
- For each module, its repository: the commit its working tree is at, and whether that tree is
  clean.
- The declared target vocabulary of `common.schema.json#/$defs/target`.

## Outputs

- A lock record per module: source commit, source tag where one names that commit, semantic schema
  version, `semantic_core` version, compiler version, and the artifact digest.
- A target record per declared target: its state, and its coordinates when it has any.
- Diagnostics for an incompatible lock, a missing generated target, and an unknown schema major.

## Behavior

- The lock record SHALL identify the source by commit, and SHALL carry a tag only in addition to
  that commit, never instead of it, because a tag can be moved and a commit cannot.
- The lock record SHALL carry the `semantic_core` version the module declares, so that two modules
  compiled against different kernel versions are distinguishable in the catalog rather than only in
  their manifests.
- Where a module's repository has uncommitted changes, the lock record SHALL record it `clean:
  false` and SHALL NOT omit the module: a dirty tree is a fact about the resolution, and dropping
  the module would silently shrink the catalog.
- A declared target with no generated package SHALL be recorded `missing`, naming the target and the
  issue that owns it, and SHALL NOT be omitted from the target list.
- The resolution SHALL NOT substitute a package whose fingerprint disagrees with the lock. A
  mismatched package SHALL be reported `incompatible` and SHALL NOT be used.
- The resolution SHALL treat a schema major it does not recognise as `unknown-major` and SHALL NOT
  fall back to the nearest recognised major.
- A module declaring no `semantic` block SHALL resolve as `dynamic-only` and SHALL remain usable;
  the catalog carries compiled and dynamic-only modules together.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-101-AC-1 | A lock record carries the source commit, and carries a tag only alongside it. | Test |
| FR-101-AC-2 | Two resolutions of an unchanged environment produce byte-identical lock records. | Test |
| FR-101-AC-3 | A module whose repository is dirty is recorded `clean: false` and is still present. | Test |
| FR-101-AC-4 | A declared target with no generated package is recorded `missing` with its owning issue, not omitted. | Test |
| FR-101-AC-5 | A package whose fingerprint disagrees with the lock is reported `incompatible` and is not substituted. | Test |
| FR-101-AC-6 | An unrecognised schema major is reported `unknown-major` with no fallback to a recognised one. | Test |
| FR-101-AC-7 | A module with no `semantic` block resolves `dynamic-only` and remains in the catalog. | Test |
| FR-101-AC-8 | The lock record carries the `semantic_core` version each module declares. | Test |
| FR-101-AC-9 | Resolution changes no byte of any module repository. | Test |
| FR-101-AC-10 | Resolution performs no network access. | Static |

## Dependencies

- The semantic module contract (`agent-ix/quoin#293`), for the `semantic` block this reads.
- `agent-ix/filament-core-data#11`, which owns the generated package coordinates. Two of its four
  targets are blocked by `agent-ix/filament-core-data#80` and `#81`, which is why a missing target
  is a recorded state here rather than a precondition.
