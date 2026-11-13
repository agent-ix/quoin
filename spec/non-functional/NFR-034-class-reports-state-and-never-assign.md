---
id: NFR-034
title: "Class floors and class obligations state what people wrote and never assign a class"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-160"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-161"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---

# NFR-034: Class floors and class obligations state what people wrote and never assign a class

## Statement

Quoin SHALL read every software safety class from the Quire payload as a person wrote it.

Quoin SHALL NOT assign, default, infer, raise or write a class in any output, file or store (FR-160, FR-161).

Quoin SHALL return the same findings and rows, in the same order, for the same payload, policy and store.

## Scope

- Applies to: `quoin evidence audit --class-policy`, `quoin matrix --class-policy` and `quoin class obligations`.
- Not applied to: the class policy file, which a person writes.

## Rationale

A class assigned by a tool is a safety decision nobody made. A report that quietly raises a requirement to its item's class
would hide the very inconsistency Quire reports (quire-rs FR-091).

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Class values appearing in output that no input states | 0 | 0 | Test |
| Files or store records written by the three commands | 0 | 0 | Test |
| Differing outputs across repeated runs over identical inputs | 0 | 0 | Test |

## Verification

The first metric is checked by FR-160-AC-9 and FR-161-AC-7, which fix an item's class away from a requirement's. The second and
third are checked by FR-160-AC-12 for the audit and the matrix and by FR-161-AC-9 for the report.
