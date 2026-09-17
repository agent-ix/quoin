---
id: FR-006
title: ConfigVersion
object: entity
type: FR
---

# FR-006: ConfigVersion

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| id | UUID | 1 | identity |

## Relationships

| Name | Verb | Target | Multiplicity |
|------|------|--------|--------------|
| overlay | references | FR-005 | 1..1 |
| entries | contains | FR-007 | 0..* |
| predecessor | references | FR-006 | 0..1 |
