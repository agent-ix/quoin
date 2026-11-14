---
id: CTRL-001
title: "RefreshTokenLifecycle"
type: control
object: control
relationships:
  - target: FR-001
    type: implements
---
# [CTRL-001] RefreshTokenLifecycle

## Mappings

| Set | Clause |
|---|---|
| nist/sp800-53@5.2.0 | AC-2 |

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
context RefreshTokenLifecycle
inv EffectivenessIsStated:
  self.effectiveness->notEmpty()
```
