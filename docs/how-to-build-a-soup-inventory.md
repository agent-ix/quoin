---
title: Build and review a software inventory
type: how-to
description: Promote an SBOM into project records and review its support and advisory status.
---

# Build and review a software inventory

From a project root, turn an SBOM into software inventory records and inspect their reported status.

## Before you start

Use Quoin 0.38.2. Prepare an SBOM in JSON and a clause-binding report for the project. Quoin reads the binding report and recorded inputs; it does not create the SBOM.

Follow these steps:

1. Promote the SBOM components into inventory records.
2. Render and review the inventory report.

## Create inventory records

```sh
quoin soup promote build/sbom.json
```

Quoin creates one record for each supported package URL. It leaves existing records unchanged and prints the created-record count.

## Review the report

```sh
quoin soup report \
  --repo . \
  --binding binding.json \
  --as-of 2026-11-25T00:00:00Z
```

The report has one row per inventory record. Unknown support and advisory data remain marked unknown, so missing inputs stay visible.

To keep the result for automated review, add `--json` to the report command.

## Result

The command prints the inventory rows and the stated support and advisory data. Missing data appears as unknown or not stated.
