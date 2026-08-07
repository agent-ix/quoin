---
id: StR-001
title: "Authors run spec work from a standalone CLI"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-001"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-004"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/NFR-004"
    type: "satisfied_by"
---

# StR-001: Authors run spec work from a standalone CLI

## Stakeholder Need

Spec-authoring agents and their operators require a single, self-contained `quoin`
command that they can install on its own and run to start every spec task —
inspecting the catalog, building an authoring contract, managing modules, and
launching workflows — so that adopting spec-driven development requires only this
one tool on `PATH`.

## Rationale

The audience for spec authoring is an agent (and the human guiding it) working
inside an arbitrary repository. Requiring the whole `ix` platform CLI to be
present just to author a spec raises the cost of adoption and couples spec work
to unrelated platform machinery. A focused, independently installable CLI keeps
the entry cost to "install one package", which is what makes the workflow usable
in any repo, including ones outside the IX platform.

## Validation Criteria


| ID | Criteria | Validation |
|----|----------|------------|
| StR-001-VC-1 | `quoin` installs as a single package and runs its catalog, authoring, plugin, workflow, and update surfaces using only its own runtime dependencies, resolving its configuration root from a flag or a sensible default without any platform-wide setup. | Demonstration |

Satisfaction is demonstrated by installing the package alone and exercising each command surface successfully.

## Stakeholders

The primary stakeholders are spec-authoring agents (the direct operators of the
CLI) and the developers who adopt spec-driven development in their repositories.
The IX platform team is an affected party that benefits from `quoin` being usable
independently of the umbrella CLI.

## Dependencies

**Downstream**: the command-surface and configuration-resolution functional
requirements ([FR-001](../functional/FR-001-parse-command-line.md),
[FR-004](../functional/FR-004-resolve-config-root.md)) and the standalone-runtime
non-functional requirement ([NFR-004](../non-functional/NFR-004-standalone-dependencies.md)).
