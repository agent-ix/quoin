---
id: StR-006
title: "Operators keep quoin current with one command"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-022"
    type: "satisfied_by"
---

# StR-006: Operators keep quoin current with one command

## Stakeholder Need

Operators require that `quoin` shall upgrade itself to the latest published
release on request, and shall be able to report whether a newer version is
available without installing it, so that keeping the tool current is a single
deliberate action rather than a manual package-management chore.

## Rationale

A spec-authoring CLI improves over time, and authors benefit from running a recent
version, but they should not have to remember the package coordinates or registry
to upgrade. A built-in update command — with a check-only mode for the cautious —
makes staying current trivial and predictable, and lets the same command serve
both public releases and local development snapshots by choice of registry.

## Validation Criteria


| ID | Criteria | Validation |
|----|----------|------------|
| StR-006-VC-1 | Running the update command compares the installed version to the latest published version and upgrades in place, while a check-only invocation reports availability without changing anything, and an explicit registry can be selected. | Demonstration |

Satisfaction is demonstrated by a check that reports an available upgrade and an update that installs it.

## Stakeholders

The primary stakeholders are operators and agents running `quoin`; the release
maintainers are an affected party who publishes the versions the command resolves.

## Dependencies

**Downstream**: the self-update functional requirement
([FR-022](../functional/FR-022-self-update.md)). The version comparison and
install are delegated to `@agent-ix/ix-cli-core`.
