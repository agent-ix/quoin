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

Quoin SHALL take every actor, authority, instant, revision, digest and approval reference of a fact from a person's input.

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
| Files written by the command | 0 | 0 | Test |

## Verification

The first is checked by FR-173-AC-1 and FR-173-AC-5, the second by FR-173-AC-2, and the third by a directory listing before and after a run.
