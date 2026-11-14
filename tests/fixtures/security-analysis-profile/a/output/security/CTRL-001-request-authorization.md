---
id: CTRL-001
title: "RequestAuthorization"
type: control
object: control
relationships:
  - target: FR-002
    type: implements
---
# [CTRL-001] RequestAuthorization

## Mappings

| Set | Clause |
|---|---|
| nist/sp800-53@5.2.0 | AC-2 |
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
context RequestAuthorization
inv EffectivenessIsStated:
  self.effectiveness->notEmpty()
```
