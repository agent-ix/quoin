---
type: master-requirements
name: {{ cookiecutter.repo_name }}
org: {{ cookiecutter.org }}
component_type: semantic-module
implementation_language: typespec
tags:
  - filament-module
  - semantic-module
  - {{ cookiecutter.module_kind }}
depends_on: []
standards_alignment:
  - iso-iec-ieee-29148
security_critical: false
---

# Master Requirements Specification

## Purpose

This document specifies **{{ cookiecutter.repo_name }}**, a Quire semantic
module. It states what the repository itself must carry and guarantee: one
declaration schema per exported type, a manifest that declares the
semantic-module contract at version 1.0.0, authoring skeletons in the mapped
Markdown forms, fixtures that exercise every refusal, and a verification suite
that fails rather than skips. Quoin installs the module, Quire validates
artifacts against it, and the shared compiler reads its emitted schemas; all
three must be able to read one declaration rather than three descriptions of it.

## Scope

### In Scope

- The `semantic` manifest block and the `{schema, digest}` reference for every exported type.
- The TypeSpec source and the deterministic emit pipeline that produces the schemas.
- One authoring skeleton per exported type in the typed-table form, its `sysml` alternate, and its `ocl` invariants.
- One negative fixture per failure mode the schemas refuse, and a legacy-form fixture.
- The verification suite and the repository gate.
- The public {{ cookiecutter.license }} packaging surfaces, Python and npm.

### Out of Scope

- **This module's domain vocabulary.** The generated repository ships worked examples of the SHAPE. Which types this module declares, what fields they carry and what their invariants say is its maintainer's specification to write; the template supplies none of it and this document does not pre-empt it.
- **Emitters for targets other than JSON Schema.** `semantic.targets` may declare `rust`, `typescript`, `python-pydantic-v2` or `python-dataclass`; no emitter produces them today, and the Test Matrix records that as a `🚧` row rather than claiming coverage.
- **Publication.** Tagging, publishing and adding the catalog entry are decisions documented in `docs/catalog-entry.md`, not obligations of this specification.
- **The Quire engine's behaviour.** This module declares a contract; `agent-ix/quire-rs` implements the extraction that reads it.

## System Overview

### System Description

A Filament module is data, not a program. This repository holds a TypeSpec source
describing each exported type's declaration record, the JSON Schemas emitted from
it, a manifest binding each type to its schema by path and digest, and the
Markdown skeletons that show how an artifact populates one. Nothing here runs at
a consumer's request; everything here is read.

The one moving part is the emit pipeline: `make schemas` compiles the TypeSpec
source with the official emitter, keeps this module's namespace, writes absolute
references, and rewrites the manifest's digests from the emitted bytes. That
pipeline is the reason a schema URL names exactly one immutable byte sequence.

### Intended Users

- **Quoin**, which installs this module into a local catalog and rejects a manifest it cannot read.
- **Quire**, which validates artifacts against the declarations this module contributes.
- **The shared compiler**, which reads the emitted schemas when it generates packages.
- **Artifact authors**, who follow the skeletons.
- **This module's maintainer**, who replaces the worked examples with real types.

## Requirements Architecture

One stakeholder requirement fixes the need — typed declarations a consumer can
read. One user story carries it into practice. Three functional requirements
allocate the obligations: the manifest declares the contract (FR-001), the
schemas are emitted deterministically from the source (FR-002), and the authoring
forms are demonstrated while their refusals are exercised (FR-003). One
non-functional requirement constrains all three: the suite reports zero skips
(NFR-001), because a green run that ran nothing is the failure mode this module's
verification exists to prevent.

### Stakeholder Requirements

- [StR-001](./stakeholder/StR-001-typed-declarations-consumers-can-read.md) — consumers read typed declarations rather than prose.

### User Stories

- [US-001](./usecase/US-001-declare-a-type-once.md) — declare a type once and have every surface read it.

### Functional Requirements

- [FR-001](./functional/FR-001-declare-the-semantic-contract.md) — the manifest declares the contract and references each schema by path and digest.
- [FR-002](./functional/FR-002-emit-schemas-deterministically.md) — schemas are emitted from the TypeSpec source, reproducibly.
- [FR-003](./functional/FR-003-authoring-forms-and-fixtures.md) — the authoring forms are demonstrated and their refusals are exercised.

### Non-Functional Requirements

- [NFR-001](./non-functional/NFR-001-verification-without-skips.md) — the suite fails rather than skips when a tool is absent.

## References

- ISO/IEC/IEEE 29148 — Requirements engineering.
- The semantic-module contract, `agent-ix/quoin` FR-070 through FR-075.
- `filament-core-data` ADR-0005 — TypeSpec is the structural schema source.
- `@agent-ix/semantic-core` {{ cookiecutter.semantic_core_version }} — the shared declaration grammar.
- `agent-ix/quire-rs#392` — publishing the engine wheel this module's suite needs.
- This repository's `README.md`, `CONTRIBUTING.md` and `toolchain.yaml`.
