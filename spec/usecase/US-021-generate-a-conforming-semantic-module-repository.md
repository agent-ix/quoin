---
id: US-021
title: "Generate a conforming semantic-module repository"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-008"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-020"
    type: "depends_on"
---

# US-021: Generate a conforming semantic-module repository

## Story

**As a** maintainer starting a new Quire semantic module
**I want** to generate the repository from one maintained template that already carries the semantic-module contract
**So that** the first commit is a repository that already validates, packages, and verifies itself, and my remaining work is my module's own vocabulary.

## Context

`spec-objects-business` and `spec-artifacts-iso` completed this migration by hand
and five more modules are in flight. Both hand migrations converged on the same
shape: a TypeSpec source importing `@agent-ix/semantic-core`, a deterministic
emit script that writes one JSON Schema per declared type and rewrites the
manifest digests textually, a `semantic` manifest block naming the contract and
package identity, typed authoring skeletons with a negative fixture per failure
mode, and a suite that fails rather than skips when the engine is absent. Nothing
in that shape is business- or ISO-specific, and nothing in it is discoverable by
reading one repository — it was arrived at twice, separately.

The alternative on offer today is copying a module repository. That carries the
source module's vocabulary, its navigation category, its frozen compatibility
baseline, and — where the source predates the contract — its
`data_schema: {type: object}` placeholder.

## Acceptance Examples (Illustrative)

### US-021-EX-1: An object module comes out validating

- **Given** a maintainer answering the template's prompts with an organization, repository name, and `module_kind: object`
- **When** the template renders and the maintainer runs the rendered repository's own gate
- **Then** the schemas emit from the TypeSpec source, the manifest digests match, `quire validate` passes over the rendered `spec/` tree, and the suite is green — with no file hand-edited after rendering

### US-021-EX-2: The engine's absence is a failure, not a pass

- **Given** a rendered repository in an environment where the Quire wheel exposing `extract_semantic` is not installed
- **When** the maintainer runs the suite
- **Then** the semantic rows fail, naming `make dev-quire` and the tracking issue, and no row reports as skipped

### US-021-EX-3: A skeleton says the same thing twice, and a negative fixture says it once

- **Given** the rendered object-module skeleton with its typed `## Properties` table and its `sysml`-fence alternate
- **When** both are extracted
- **Then** they yield the same declarations, and the rendered negative fixture for each declared failure mode is refused for its own distinct reason

### US-021-EX-4: A placeholder does not survive rendering

- **Given** a rendered repository of any variant
- **When** its tree is scanned
- **Then** no unresolved template token, no placeholder organization, no absolute path from the generating machine, no credential, and no private-registry publication default remains, and one license string is used everywhere

## Options (Exploratory)

The template could be a standalone repository, a directory inside Quoin, or a
generator command. Variants could be three copied templates or one core with
conditional rendering. The generated schema payload could be committed or emitted
at install time. The functional requirements settle each.

## Constraints (Contextual)

The template depends on versioned shared tooling and never carries a copy of the
emitter or the runtime. No existing module repository is recreated or normalized
by this story. The rendered repository is public and AGPL-3.0-or-later, with no
silent fallback to another license.

## Dependencies (Contextual)

Depends on [US-020](./US-020-declare-a-semantic-module-contract-once.md) and the
contract it drives ([FR-070](../functional/FR-070-semantic-module-manifest-extension.md)
through [FR-075](../functional/FR-075-semantic-package-exports-and-locks.md)), on
ADR-0005 fixing TypeSpec as the structural source, and on
`@agent-ix/semantic-core`.

## Priority and Risk (Informative)

P0. The principal risk is a template that is maintained but never instantiated,
so its output drifts from the contract silently; the second is a template that
bakes one module's vocabulary into every module that follows it.

## Traceability (Informative)

Drives [FR-076](../functional/FR-076-semantic-module-template-variants.md) through
[FR-083](../functional/FR-083-template-render-self-tests.md), constrained by
[NFR-018](../non-functional/NFR-018-rendered-output-hygiene.md) and
[NFR-019](../non-functional/NFR-019-deterministic-rendering.md).
