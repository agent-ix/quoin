---
id: CTRL-001
title: "AccessEnforcement"
type: control
object: control
relationships:
  - target: FR-001
    type: implements
---
<!-- Control template for spec-security-analysis. One control per capability the
     component implements. Take the skeleton with `quoin write <repo> --types control`
     and fill it as below; delete this comment.
     - relationships: one `implements` entry per FR, NFR or StR the control answers.
     - Mappings: one row per clause. Set is the installed clause set key
       (authority/id@version), Clause is the clause id inside it. Write a row
       only when the set is installed and holds the clause. No titles or text.
     - Effectiveness: `not_assessed` unless a person supplied another value.
     - Properties and Invariants: as in the control skeleton. -->
# [CTRL-001] AccessEnforcement

## Mappings

| Set | Clause |
|---|---|
| nist/sp800-53@5.2.0 | AC-3 |
| owasp/asvs@5.0.0 | V8.2.1 |

## Effectiveness

`not_assessed`

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| control_id | UUID | 1..1 | identity |
| effectiveness | String | 1..1 | minLength: 1 |

## Invariants

### EveryRequestIsAuthorized

```ocl
context AccessEnforcement
inv EveryRequestIsAuthorized:
  self.effectiveness->notEmpty()
```
