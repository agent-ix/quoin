---
id: negative-005
title: "NoteInTwoForms"
type: note
expect: semantic.properties.both-forms-present
because: "one artifact carries one Properties form; the alternate is a separate file"
---
# [negative-005] NoteInTwoForms

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| summary | String | 1..1 | minLength: 1 |

```sysml
attribute summary : String[1..1] { minLength: 1 }
```
