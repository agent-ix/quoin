---
id: note-001
title: "Handover Note"
type: note
---
<!-- note authoring skeleton, alternate Properties form. Declares exactly the
     same fields as note.md, authored as one ```sysml``` fence instead of the
     typed table. One artifact carries ONE form; the alternate is a separate
     file, never a second block in the same document. -->
# [note-001] Handover Note

## Properties

```sysml
attribute summary : String[1..1] { minLength: 1, maxLength: 2000 }
attribute author : String[1..1] { minLength: 1 }
attribute recorded_at : Timestamp[1..1]
attribute supersedes : String[0..1]
```

## Invariants

The clauses a Handover Note declaration enforces.

### SupersededNoteIsNotItself

```ocl
context HandoverNote
inv SupersededNoteIsNotItself:
  self.supersedes->notEmpty() implies self.supersedes <> self.id
```
