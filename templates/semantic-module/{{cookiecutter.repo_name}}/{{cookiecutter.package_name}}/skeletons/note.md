---
id: note-001
title: "Handover Note"
type: note
---
<!-- note authoring skeleton ({{ cookiecutter.repo_name }}). Fill every section
     with substantive content. Contract (the manifest's body_extraction asserts
     it, and `make test` proves it):
     - Frontmatter MUST carry id, title, type: note.
     - "## Properties" (H2, required): one typed row per declared property,
       header exactly `Field | Type | Multiplicity | Constraints`. A note carries
       no identity row: a document's identity is its frontmatter `id`.
     - "## Invariants" (H2, optional): one `### <clauseId>` per clause, each
       owning exactly one ```ocl``` fence.
     REPLACE the vocabulary. `Handover Note` is a worked example of the SHAPE. -->
# [note-001] Handover Note

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| summary | String | 1..1 | minLength: 1, maxLength: 2000 |
| author | String | 1..1 | minLength: 1 |
| recorded_at | Timestamp | 1..1 | |
| supersedes | String | 0..1 | |

## Invariants

The clauses a Handover Note declaration enforces.

### SupersededNoteIsNotItself

```ocl
context HandoverNote
inv SupersededNoteIsNotItself:
  self.supersedes->notEmpty() implies self.supersedes <> self.id
```
