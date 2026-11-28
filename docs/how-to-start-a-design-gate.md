---
title: Start a design gate
type: how-to
description: Start a review workflow for a rule that crosses a system boundary.
---

# Start a design gate

From the project root, start the design-gate workflow for a proposed boundary-crossing rule.

## Before you start

Use Quoin 0.38.2 and install the Quoin design-gate skill. Prepare the target rule or decision and keep the run id for the resulting workflow.

Follow these steps:

1. Start the workflow for a project file.
2. Review the workflow result and recorded evidence.

## Start the workflow

```sh
quoin design-gate \
  --path /absolute/path/to/design-gate-skill \
  --target docs/semantic-module-architecture/adr/0001-authority-by-concern.md \
  --id design-gate-001 \
  --json
```

Quoin passes the target and run id to the installed workflow. Review the workflow output and its recorded evidence before using its recommendation.

## Result

The command prints the workflow result as JSON and retains the run output. A design gate supports review of the proposed boundary; a person remains responsible for accepting the design.
