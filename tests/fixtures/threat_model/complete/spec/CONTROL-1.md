---
id: CONTROL-1
title: Encryption and access control
type: control
object: control
safeguards: [encrypt-in-transit]
relationships:
  - type: mitigates
    target: THREAT-1
  - type: protects
    target: CONN-1
  - type: protects
    target: SINK-1
---
# Encryption and access control

## Effectiveness

`effective`

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| control_id | UUID | 1..1 | identity |
| effectiveness | String | 1..1 | minLength: 1 |

## Invariants

The control safeguards the in-scope connection and sink.

### ControlIsStated

```ocl
context EncryptionAndAccessControl
inv ControlIsStated: true
```
