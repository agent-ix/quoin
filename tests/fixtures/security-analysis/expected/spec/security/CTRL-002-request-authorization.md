---
id: CTRL-002
title: "RequestAuthorization"
type: control
object: control
relationships:
  - target: FR-002
    type: implements
---
# [CTRL-002] RequestAuthorization

## Mappings

| Set | Clause |
|---|---|
| nist/sp800-53@5.2.0 | AC-3 |
| nist/sp800-53@5.2.0 | AC-6 |
| owasp/asvs@5.0.0 | V8.2.1 |

## Effectiveness

`not_assessed`

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| control_id | UUID | 1..1 | identity |
| effectiveness | String | 1..1 | minLength: 1 |

## Invariants

### NoRoleNoForward

```ocl
context RequestAuthorization
inv NoRoleNoForward:
  self.effectiveness->notEmpty()
```
