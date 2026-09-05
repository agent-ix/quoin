---
id: US-023
title: "Lock the semantic catalog to exact revisions"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
---

# US-023: Lock the semantic catalog to exact revisions

## Story

**As a** maintainer installing a semantic module into a repository
**I want** the catalog to record exactly which module revision, kernel version and generated package
it resolved, and to refuse a package that disagrees with that record
**So that** an artifact produced today can still be attributed to the contract that produced it, and
a mismatched package fails loudly instead of being substituted.

The story states what the maintainer needs. It does not say what a lock file looks like or how
resolution is implemented; those belong to the requirements it drives.

## Context

Two failures in the current wave are the reason this is a story rather than an implementation
detail.

`agent-ix/quoin#350`: this repository's own verification stack pins a `quire-rs` revision that the
campaign then moved past. The stack sits in front of the whole suite, so the stale pin did not
degrade one check — it removed every check at once, and it did so by refusing rather than by
passing. That is the good direction to fail in, and it still meant `make test` proved nothing here
until somebody noticed.

`agent-ix/quire-rs#405`: `IX_FILAMENT_MODULES_PATH` adds to the default install root rather than
replacing it, and resolution is first-wins. A measurement pinned ten modules and the engine
answered from whichever copy it found first. The verdicts were sound; their attribution to the
pinned revisions was not, and nothing in the output said so.

Both are the same shape at different scales: a resolution that cannot state what it resolved. The
maintainer's need is not "a lock file" — it is the ability to answer, later, which contract produced
an artifact.

## Acceptance

| ID | Criterion |
| --- | --- |
| US-023-AC-1 | A maintainer can read, from catalog output alone, which module revision and kernel version a resolution used. |
| US-023-AC-2 | A generated package whose fingerprint disagrees with the lock is refused rather than substituted. |
| US-023-AC-3 | A declared target with no generated package is visible as missing, with the issue that owns it. |
| US-023-AC-4 | A module with no semantic block still installs, so the transition does not break existing modules. |

## Out of scope

Publishing any generated package: that passes `agent-ix/quoin#290`, a human sign-off that has not
moved. Upgrading a consuming repository: the ticket's safety gate says catalog pin changes stay
additive and advisory, and consuming repositories are never auto-upgraded.
