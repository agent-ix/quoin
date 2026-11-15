---
id: NFR-039
title: "Dossier generation is deterministic and writes only what the records state"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-188"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-189"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-190"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-192"
    type: "constrains"
---

# NFR-039: Dossier generation is deterministic and writes only what the records state

## Statement

`quoin dossier generate` and `quoin dossier check` SHALL give byte-identical Markdown for the same records and options, where the options include the order of `--clause-binding` and the index order of a pack.

`quoin dossier generate` and `quoin dossier check` SHALL order output by the dossier order and gap order of FR-188, except where FR-192 keeps a supplied order.

`quoin dossier generate` and `quoin dossier check` SHALL NOT read the wall clock or the network.

`quoin dossier generate` and `quoin dossier check` SHALL NOT write a criticality, risk, compliance or acceptability value that no source record states.

## Scope

- Applies to: `generate` and `check` and the files they write or compare, and to the regeneration that `export` performs before `quire export`.
- Not applied to: `quire export`'s own output (quire-cli FR-037).

## Rationale

A reviewer relies on the documents matching the commit. A value the generator supplied would read as a decision someone recorded.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Byte differences between two runs on one commit | 0 | 0 | Test |
| Clock or network reads in the dossier code path | 0 | 0 | Inspection |
| Criticality, risk, compliance or acceptability values with no source record | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-039-AC-1 | Two runs over the same commit give byte-identical files | Test |
| NFR-039-AC-2 | Inspection of the dossier code path finds no wall-clock call, and analysis of its dependency graph finds no network crate | Inspection |
| NFR-039-AC-3 | For a fixture whose records state no criticality, risk, compliance or acceptability, none of those values appears in any generated file | Test |
| NFR-039-AC-4 | Fixture documents created in a different order on disk give the same bytes, and swapping two `--clause-binding` options changes only the order of the two RMM tables | Test |
| NFR-039-AC-5 | A `generate` run that selects only non-RMM documents (`srs`, `architecture`, `trace-matrix`) with no `--as-of` writes no time into any file | Test |

## Verification

NFR-039-AC-2 is verified by Inspection and static analysis of the source and the dependency graph, because a test cannot make the clock unavailable to a process. The other criteria are tests over the generated files.
