---
id: US-005
title: "Start a gated spec workflow"
type: US
relationships:
  - target: "ix://agent-ix/quoin/FR-020"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---

# US-005: Start a gated spec workflow

## Story

**As a** user running the spec lifecycle
**I want** `quoin` to launch the spec-domain workflow and `ix-flow` to manage phase progression and human gates
**So that** the workflow engine stays separate from the catalog and authoring surface.

## Context

A user or agent starts review or planning workflow runs from the spec CLI and
continues execution through `ix-flow`. (`quoin matrix` is not a workflow: it
renders the evidence-backed matrix in-process, per FR-115.)

The expected command shape is:

```bash
quoin review --target spec/
ix-flow status <run-id>
ix-flow resume <run-id>
```

The workflow should reference catalog-defined types instead of redefining the
artifact/object vocabulary inside the workflow definition.

## Dependencies

- `ix-flow` is installed separately.
- Workflow definitions are available to `quoin`.
