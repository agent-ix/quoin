---
id: negative-003
title: "NoteWithoutProperties"
type: note
expect: semantic.locator.required-section-missing
because: "the manifest marks the Properties section required for a note"
---
# [negative-003] NoteWithoutProperties

## Invariants

### SupersededNoteIsNotItself

```ocl
context HandoverNote
inv SupersededNoteIsNotItself:
  self.supersedes->notEmpty() implies self.supersedes <> self.id
```
