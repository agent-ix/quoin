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
| nist/sp800-53@5.2.0 | IA-5 |
| nist/sp800-53@5.2.0 | AC-12 |
| owasp/asvs@5.0.0 | V7.2.1 |

## Effectiveness

`not_assessed`

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| control_id | UUID | 1..1 | identity |
| effectiveness | String | 1..1 | minLength: 1 |

## Invariants

### ExchangeInvalidatesThePresentedToken

```ocl
context RefreshTokenRotation
inv ExchangeInvalidatesThePresentedToken:
  self.effectiveness->notEmpty()
```
