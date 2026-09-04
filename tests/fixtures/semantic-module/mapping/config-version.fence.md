---
id: FR-006
title: ConfigVersion Entity
object: entity
type: FR
---

# FR-006: ConfigVersion Entity

## Description

The service SHALL persist configuration versions as immutable `ConfigVersion` entities.

## Properties

```sysml
attribute id : UUID[1] { identity }
attribute versionNumber : Integer[1] { min: 1 }
attribute data : JsonObject[1]
attribute hash : String[1] { nonEmpty }
ref item parent : ConfigVersion[0..1]
attribute createdAt : Timestamp[1]
attribute createdBy : String[1] { maxLength: 64 }
```

## Relationships

- `overlay`: belongs_to → ConfigOverlay (FR-005)

## Invariants

### immutable

```ocl
context ConfigVersion inv immutable: self.versionNumber = self.versionNumber@pre and self.data = self.data@pre and self.hash = self.hash@pre
```
