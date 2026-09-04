---
id: TASK-045
title: "Template core, input contract and conformance contract"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-076"
    type: references
  - target: "ix://agent-ix/quoin/NFR-020"
    type: references
  - target: "ix://agent-ix/quoin/TC-1400"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1401"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1402"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1403"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1404"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1405"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1406"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1407"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1452"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1453"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1454"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1463"
    type: verifies
---

# TASK-045: Template core, input contract and conformance contract

## Scope

Create `templates/semantic-module/` as one cookiecutter whose variants are a
rendering decision: `cookiecutter.json` with the declared inputs, a pre-generation
hook that refuses every invalid input before writing a file, a post-generation
hook that prunes the surfaces the chosen variant does not carry, and
`conformance.yaml` declaring the required surfaces, the residue patterns, the
pinned maintained repositories and the exemptions.

## Subtasks

- [x] `cookiecutter.json`: org, repo name, module and package names, description, author, email, version, `module_kind`, licence, semantic-core version, TypeSpec version, Python version, generated targets, imported modules, navigation category, engine floor.
- [x] `hooks/pre_gen_project.py`: refuse a non-AGPL licence, an unknown `module_kind`, a target outside the filament-core-data registry, an import without an exact version, a duplicate import identity, and a mixed variant with no imports — each naming the value, each before the first write (TC-1404..TC-1407, TC-1453, TC-1454, TC-1459).
- [x] `hooks/post_gen_project.py`: remove the artifact-only and object-only surfaces the variant does not carry, and print the next commands.
- [x] `conformance.yaml`: required paths per variant, forbidden globs, residue patterns, maintained repositories pinned by remote and full revision, and a reason for every exemption.
- [x] Record the declared floor of every external command the template and a rendered repository invoke, in one file (TC-1463).
- [x] `templates/semantic-module/README.md`: what the template renders, how to render it unattended, and what the maintainer fills in.
- [x] Exclude the template tree from this repository's prettier, eslint and vitest globs — `{{ ... }}` path segments are not JavaScript and are not this repository's spec.

## Deliverables

- A cookiecutter that renders all three variants unattended and refuses every invalid input naming it.
- A reviewable conformance contract.

## Notes

- The emit driver the rendered repository carries is a driver, not the emitter (FR-076-CON-1). Keeping every rendered copy byte-identical to the template's is FR-076-CON-3; extracting it into a versioned package is separate work.
