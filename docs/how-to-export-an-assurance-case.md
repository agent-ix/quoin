---
title: Export an assurance case
type: how-to
description: Render a validated assurance case as GSN or SACM for review.
---

# Export an assurance case

From the project root, export the assurance case already described by the authored records.

## Before you start

Use Quoin 0.38.2. Prepare the assurance records and their evidence references. Quoin renders those inputs and keeps unresolved parts visible.

Follow these steps:

1. Export the case as GSN JSON.
2. Export the case as SACM XMI.
3. Review the export and any mapping report.

## Export GSN

```sh
quoin assurance --repo . --gsn > build/assurance-case.json
```

The file contains the validated GSN model. Quoin returns an error if the model cannot be exported.

## Export SACM

```sh
mkdir -p build
quoin assurance --repo . --sacm > build/assurance-case.xmi
```

The file contains the SACM XMI export. To retain a separate report of content that could not be mapped, add:

```sh
quoin assurance --repo . --sacm --sacm-report build/assurance-case-report.json > build/assurance-case.xmi
```

Review the exported file and any report before using them in a submission. The export preserves the authored case; it does not approve it.

## Result

The GSN command writes JSON to standard output. The SACM command writes XMI to standard output, plus the optional mapping report at the requested path.
