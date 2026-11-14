---
id: CTRL-001
title: "CanonicalInputNormalization"
type: control
object: control
relationships:
  - target: FR-004
    type: implements
---
# [CTRL-001] CanonicalInputNormalization

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
context CanonicalInputNormalization
inv EffectivenessIsStated:
  self.effectiveness->notEmpty()
```
