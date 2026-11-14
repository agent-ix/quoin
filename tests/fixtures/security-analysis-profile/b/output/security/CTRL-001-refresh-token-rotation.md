---
id: CTRL-001
title: "RefreshTokenRotation"
type: control
object: control
relationships:
  - target: FR-001
    type: implements
---
# [CTRL-001] RefreshTokenRotation

## Mappings

| Set | Clause |
|---|---|
| owasp/asvs@5.0.0 | V1.1.1 |

## Effectiveness

`not_assessed`

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| control_id | UUID | 1..1 | identity |
| effectiveness | String | 1..1 | minLength: 1 |

## Invariants

### EffectivenessIsStated

```ocl
context RefreshTokenRotation
inv EffectivenessIsStated:
  self.effectiveness->notEmpty()
```
