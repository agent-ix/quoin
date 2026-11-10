---
id: negative-003
title: "NoteWithoutProperties"
type: note
expect: semantic.locator.required-section-missing
because: "the manifest marks the Properties section required for a note"
---
# [negative-003] NoteWithoutProperties

## Invariants

### SupersededNoteIsNamed

```quire
present(self.supersedes) implies value(self.supersedes) != ""
```
