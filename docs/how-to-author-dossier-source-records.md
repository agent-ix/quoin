---
title: Author dossier source records
type: how-to
description: Add product facts as source records and render them into a dossier.
---

# Author dossier source records

From a project root, save product facts as JSON source records, then render the dossier documents that use them.

## Before you start

Use Quoin 0.38.2. Store source records under `dossier/sources/`. Each file's directory names its record kind, and its filename matches the record id.

Follow these steps:

1. Add a product-source record.
2. Generate the document that uses it.
3. Check the rendered result.

## Add a software description

Create `dossier/sources/software_description/SW-001.json`:

```json
{
  "schema_version": 1,
  "id": "SW-001",
  "record_type": "software_description",
  "software_id": "sample-service",
  "name": "Sample service",
  "purpose": "Stores sample observations.",
  "functions": ["Accepts observations", "Displays current status"]
}
```

Add related documentation-level or device records in their matching directories. Keep every value grounded in an authored project record.

## Generate the document

```sh
quoin dossier generate \
  --scope . \
  --doc software-description \
  --version 1.0.0 \
  --out build/dossier
```

Then check that the rendered file matches the selected source records:

```sh
quoin dossier check --scope . --doc software-description --version 1.0.0 --out build/dossier
```

The check exits nonzero when a source record leaves required fields or references unstated. Read its gaps, then add authored facts when they are available.

## Result

The generated software description preserves the authored name, purpose, and functions. The report names missing or unresolved source references as gaps.
