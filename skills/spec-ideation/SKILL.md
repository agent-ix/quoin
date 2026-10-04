---
name: spec-ideation
description: Use for loose, exploratory specification drafting before formal authoring, review, matrix, or planning work.
---

# Spec Ideation

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

## Mode

This is a config-only skill. Do not create workflow state, call workflow
commands, or enforce phase gates while using it.

## Framing Moves

- State the problem in concrete user-facing terms.
- Separate users, operators, and system actors.
- Draft candidate features as options, not commitments.
- Name non-goals early when they protect scope.
- Convert uncertainty into open questions instead of blocking progress.
- Define success criteria that can later become evidence targets.

## Draft Outline

```markdown
# Draft Spec

## Problem

## Users And Actors

## Goals

## Non-Goals

## Candidate Features

## Open Questions

## Success Criteria
```

## Promotion Signal

Recommend promotion to `specify` when the draft has a clear problem, named
users or actors, scoped goals and non-goals, and at least one success criterion
that can be verified.
