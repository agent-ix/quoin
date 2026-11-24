---
id: NFR-044
title: "Self-tests run offline with repository-owned fixtures"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-215"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-216"
    type: "constrains"
---

# NFR-044: Self-tests run offline with repository-owned fixtures

## Statement

The self-test suite shall run without network access or a source checkout.

## Scope

- Applies to self-test execution for quoin and quire-cli, including embedded
  quire-rs cases, and to retained real-tool runs for engineering-assurance and
  filament-analyze.
- The customer's `HOME` directory may be empty when a self-test starts.

## Rationale

An installed tool must be testable without access to the source tree or an
external corpus, and each repository remains the source of its own test assets.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Network requests during a self-test run | 0 | 0 | Test |

## Verification

Run a self-test with network access disabled and `HOME` set to an empty
directory.
