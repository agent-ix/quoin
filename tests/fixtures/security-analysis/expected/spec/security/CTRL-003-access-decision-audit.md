---
id: CTRL-003
title: "AccessDecisionAudit"
type: control
object: control
relationships:
  - target: FR-003
    type: implements
---
# [CTRL-003] AccessDecisionAudit

## Mappings

| Set | Clause |
|---|---|
| nist/sp800-53@5.2.0 | AU-2 |
| nist/sp800-53@5.2.0 | AU-12 |
| owasp/asvs@5.0.0 | V16.2.1 |

## Effectiveness

`not_assessed`

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| control_id | UUID | 1..1 | identity |
| effectiveness | String | 1..1 | minLength: 1 |

## Invariants

### EveryDecisionIsRecorded

```ocl
context AccessDecisionAudit
inv EveryDecisionIsRecorded:
  self.effectiveness->notEmpty()
```
