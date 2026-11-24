---
id: NFR-043
title: "Self-test fixtures are owned by their tool repositories"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-215"
    type: "constrains"
---

# NFR-043: Self-test fixtures are owned by their tool repositories

## Statement

Each self-test suite shall use only test fixtures authored and maintained in
the repository that builds the tool binary being tested, and shall not copy
cases from the qa-corpus repository.

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
| Self-test cases sourced from another tool repository | 0 | 0 | Inspection |
| Self-test cases sourced from qa-corpus | 0 | 0 | Inspection |

## Verification

Inspect each tool repository's packaged test assets and confirm all cases have
an owning-repository source path and no case is copied from qa-corpus.
