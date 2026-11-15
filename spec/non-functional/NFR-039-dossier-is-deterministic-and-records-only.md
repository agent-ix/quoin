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

`quoin dossier generate` SHALL give byte-identical Markdown for the same records and options.

`quoin dossier generate` SHALL order output by requirement id, then criterion id, then element id, then clause id.

`quoin dossier generate` SHALL NOT read the wall clock or the network.

`quoin dossier generate` SHALL NOT write a criticality, risk, compliance or acceptability value that no source record states.

## Scope

- Applies to: `generate` and `check` and the files they write or compare.
- Not applied to: `export`, whose output is `quire export`'s (quire-cli FR-037).

## Rationale

A reviewer relies on the documents matching the commit. A value the generator supplied would read as a decision someone recorded.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Byte differences between two runs on one commit | 0 | 0 | Test |
| Clock or network reads during generation | 0 | 0 | Test |
| Criticality, risk, compliance or acceptability values with no source record | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-039-AC-1 | Two runs over the same commit give byte-identical files | Test |
| NFR-039-AC-2 | A run with the clock and network unavailable gives the same bytes; with no `--as-of` no time appears in the file | Test |
| NFR-039-AC-3 | For a fixture whose records state no criticality, risk, compliance or acceptability, none of those values appears in any generated file | Test |
| NFR-039-AC-4 | Fixture records supplied in a different order give the same bytes | Test |

## Verification

Each criterion is a test over the generated files.
