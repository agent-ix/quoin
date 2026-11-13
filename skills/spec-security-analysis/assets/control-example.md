---
id: CTRL-002
title: "PermissionEnforcement"
type: control
object: control
relationships:
  - target: FR-001
    type: implements
  - target: FR-002
    type: implements
  - target: FR-008
    type: implements
  - target: NFR-002
    type: implements
---
<!-- Example for a permissions library: access enforcement answered by FR-001,
     FR-002, FR-008 and the fail-safe NFR-002. -->
# [CTRL-002] PermissionEnforcement

## Mappings

| Set | Clause |
|---|---|
| nist/sp800-53@5.2.0 | AC-2 |
| nist/sp800-53@5.2.0 | AC-3 |
| nist/sp800-53@5.2.0 | AC-5 |
| nist/sp800-53@5.2.0 | AC-6 |
| owasp/asvs@5.0.0 | V8.1.1 |
| owasp/asvs@5.0.0 | V8.2.2 |

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
context PermissionEnforcement
inv EffectivenessIsStated:
  self.effectiveness->notEmpty()
```
