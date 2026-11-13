---
id: NFR-038
title: "Facts from mappings state what people wrote and never invent an attestation or a verdict"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-173"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-174"
    type: "constrains"
---

# NFR-038: Facts from mappings state what people wrote and never invent an attestation or a verdict

## Statement

Quoin SHALL take every actor, authority and instant of a fact from a person's input.

Quoin SHALL NOT mark a clause discharged from a mapping alone (FR-173).

Quoin SHALL NOT assign an effectiveness from a mapping alone (FR-173).

Quoin SHALL write no file from `facts-from-mappings`.

## Scope

- Applies to: `quoin evidence facts-from-mappings` and the skill's output.
- Not applied to: `record-discharge`, which stores what it is given (FR-133).

## Rationale

A fact generated with a made-up attestor would read as an authorised decision nobody made.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Attestation values in output that no input states | 0 | 0 | Test |
| Facts for clauses without an effective mapping control | 0 | 0 | Test |
| `effectiveness` values in the output that the corpus does not state | 0 | 0 | Test |
| Files written by the command | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-038-AC-1 | The `attestation` of every emitted fact equals the supplied object, and a missing or extended one is refused (FR-173-AC-1, FR-173-AC-5). | Test |
| NFR-038-AC-2 | No fact is emitted for a clause whose controls are not `effective` (FR-173-AC-2). | Test |
| NFR-038-AC-3 | The output holds no `effectiveness` key, and each control's `effective` comes from its own `## Effectiveness` section (FR-173-AC-1, FR-173-AC-2). | Test |
| NFR-038-AC-4 | A run writes no file (FR-173-AC-8). | Test |

## Verification

Each criterion is a test over the command's output and the working directory.
