---
id: negative-004
title: "NoteWithUntypedProperty"
type: note
expect: semantic.properties.untyped-row
because: "the typed table's Type column is not optional; an untyped row declares nothing a consumer can read"
---
# [negative-004] NoteWithUntypedProperty

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| summary | | 1..1 | minLength: 1 |
| author | String | 1..1 | |
