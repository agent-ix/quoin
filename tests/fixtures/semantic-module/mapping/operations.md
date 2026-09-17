---
id: FR-901
title: Operations
object: entity
type: FR
---

# FR-901: Operations

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| id | UUID | 1 | identity |

## Invariants

### notArchived

```quire
not (self.summary = "archived")
```

### archived

```quire
result.summary = "archived"
```

## Operations

### archive

| Param | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| reason | String | 1 | nonEmpty |
| delay | Duration [ms] | 0..1 | |

Returns: ConfigVersion[1]
Requires: notArchived
Ensures: archived
