---
id: NFR-043
title: "Self-test fixtures are owned by their tool repositories"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-229"
    type: "constrains"
---

# NFR-043: Self-test fixtures are owned by their tool repositories

## Statement

Each owning tool repository SHALL author and maintain its own self-test cases.
A consuming repository SHALL NOT copy those cases or any case from the
qa-corpus repository. A consuming binary MAY embed case assets exposed by a
linked crate at build time without copying or maintaining those assets. In
particular, quire-cli may embed quire-rs-owned cases exposed by the linked
quire-rs crate; quire-rs remains the only repository that authors and
maintains those cases.

## Scope

- Applies to test cases embedded into quoin and quire-cli binaries and to
  retained test evidence from engineering-assurance and filament-analyze.
- The qa-corpus repository remains the controlled corpus for measuring
  quire/quoin detection behavior, not a source of shipped self-test cases.

## Rationale

The installed-tool check must travel with its owning binary, while the
controlled detection corpus remains independently maintained and is not
repackaged as a self-test fixture set.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Self-test cases copied from another tool repository | 0 | 0 | Inspection |
| Self-test cases sourced from qa-corpus | 0 | 0 | Inspection |

## Verification

Inspect each tool repository's packaged test assets and confirm each case has
one owning-repository source path, embedded dependency assets are exposed by
their owning crate at build time, and no case is copied from another
repository or qa-corpus.
