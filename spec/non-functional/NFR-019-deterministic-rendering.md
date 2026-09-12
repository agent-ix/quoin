---
id: NFR-019
title: "Deterministic rendering and regeneration"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-076"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-077"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-083"
    type: "constrains"
---

# NFR-019: Deterministic rendering and regeneration

## Statement

Rendering a variant twice from the same inputs, and regenerating a rendered
repository's schemas twice from an unchanged source, SHALL each produce
byte-identical output, so that a diff in a rendered repository is a change
somebody made.

## Scope

- Applies to: the rendered file set of every variant, and the emitted schema set, `toolchain.json`, and manifest digests of a rendered repository.
- Operational context: two renders on the same machine and two renders on machines whose only difference is the working directory and the clock.
- Not claimed: byte-identity across operating systems. The rendered tree is written with LF endings by a declared `.gitattributes`, which makes the committed bytes identical, but the working-tree bytes a checkout produces are the platform's business and this requirement does not measure them.

## Rationale

Non-determinism defeats every downstream check that compares bytes. The digest
contract of FR-073 assumes the emitted bytes are a function of the source; the
drift gate of FR-077 assumes a re-emission that changes nothing produces no diff;
and the conformance gate of FR-083 assumes two renders can be compared. A
timestamp, an unsorted map iteration, or an embedded absolute path breaks all
three at once, and breaks them quietly.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Differing files between two renders of one variant | 0 | 0 | Test |
| Differing bytes between two schema emissions from an unchanged source | 0 | 0 | Test |
| Rendered files carrying a rendering timestamp | 0 | 0 | Test |
| Rendered files carrying an absolute path from the rendering machine | 0 | 0 | Test |

## Verification

Quoin's gate renders each variant twice into two temporary directories and
compares the trees byte for byte, then runs the rendered emit command twice over
one rendered variant and compares the emitted schemas, `toolchain.json`, and the
manifest digest lines.

## Dependencies

- **Upstream**: [FR-077](../functional/FR-077-generated-schema-emission.md)
- **Downstream**: [FR-083](../functional/FR-083-template-render-self-tests.md)
