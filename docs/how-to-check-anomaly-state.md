---
title: Check anomaly state before release
type: how-to
description: Compare anomaly records with repository history and review carried findings.
---

# Check anomaly state before release

From the repository root, compare each anomaly record with its git history and review the findings that remain open.

## Before you start

Use Quoin 0.38.2 and run the commands from a Git worktree containing the anomaly records.

Follow these steps:

1. Check each anomaly against repository history.
2. Read the carried-anomaly report.
3. Apply the release gate.

## Check recorded changes

```sh
quoin anomaly check spec
```

Quoin prints one diagnostic for each history mismatch. A successful check has no error diagnostics.

## Review the release report

```sh
quoin anomaly report spec
```

The report groups carried anomalies by affected hazard and requirement. It includes the status and release rationale recorded in each anomaly document.

## Apply a release gate

To fail when an anomaly remains unassessed or an impacting anomaly has no release rationale, run:

```sh
quoin anomaly report spec --fail-on unassessed --fail-on unjustified-impacting
```

The command returns a nonzero status when either condition is present. Review the report before changing an anomaly's authored state.

## Result

The report lists each carried anomaly and its recorded rationale. A gated report returns a nonzero status when an anomaly matches either selected condition.
