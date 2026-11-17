---
title: Analyze formal rules from a checked package
type: how-to
---

Use this procedure when a formalization draft must be compared with approved
rules. It accepts the package produced by QSL's compile command.

## Before you start

Prepare a QSL compile request and start the `formalize` workflow from the
`skills/formalize-requirement` directory.

## Compile the package

1. Write QSL's checked package to a file:

```bash
fsl compile <compile-request.json> --output /tmp/formal-rules.checked.json
```

The file uses the `filament.checked-package/v2` format.

## Set the formalization request

2. Set `formalization_request.analysis_package` to the checked package's
   absolute path. Set `formalization_request.drafted_symbols` to the declaration
   under review, then run the workflow:

```bash
ix-flow run formalize --path <absolute-skill-directory> --id <run-id> --json
```

The request identifies the draft. Quoin uses approved declarations as peer
claims and selects only draft spans for analysis. It lowers the admitted claims
to `filament.contract-package/v1` for `filament-analyze run-package`.

## Review the analysis

3. Read the differential report and its selected and excluded claim lists
   before continuing the workflow.

## Result

The workflow retains the differential report and names selected and excluded
claims. Unsupported package versions and failed admission stop with a useful
refusal before an analyzer report is written.
