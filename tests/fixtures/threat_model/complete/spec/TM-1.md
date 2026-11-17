---
id: TM-1
title: Complete threat model fixture
type: threat_model
object: threat_model
method: stride_per_interaction
reviewed_by: A. Person
relationships:
  - type: scopes
    target: PART-1
  - type: scopes
    target: BOUND-1
  - type: scopes
    target: CONN-1
  - type: scopes
    target: SINK-1
  - type: considers
    target: THREAT-1
---
# Complete threat model fixture

## Scope

The complete fixture covers the connection and database sink.

## Assumptions

The connection uses its recorded controls.

## Out of scope

No other components are included.

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| threat_model_id | UUID | 1..1 | identity |
| method | String | 1..1 | minLength: 1 |
| reviewed_by | String | 1..1 | minLength: 1 |
