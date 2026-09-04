---
id: StR-001
title: "Consumers need typed declarations rather than prose"
type: StR
verification_method: test
relationships:
  - target: "ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/FR-001"
    type: "satisfied_by"
  - target: "ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/FR-002"
    type: "satisfied_by"
---
# StR-001: Consumers need typed declarations rather than prose

## Stakeholder Need

Consumers of this module require that every type it exports shall carry a machine-readable
declaration schema, identified by an immutable URL and a digest, so that a
generator, a validator and a formal-clause checker all read the same declaration
instead of each parsing prose in its own way.

## Rationale

Before the semantic-module contract, an object archetype declared
`data_schema: {type: object}` and its Properties section was free text. A field's
type lived in prose, so nothing downstream could type it, and two consumers that
read the same document could disagree without either being wrong. A schema
referenced by path and digest makes the declaration one artifact with one
identity.

## Validation Criteria

| ID | Criteria | Validation |
|----|----------|------------|
| StR-001-VC-1 | Every exported type carries an emitted schema referenced by path and digest, and no exported type carries an inline placeholder contract. | Test |
| StR-001-VC-2 | A declaration extracted from an authoring skeleton validates against that type's emitted schema. | Test |

## Stakeholders

The primary stakeholders are the tools that read this module: Quoin at install
time, Quire at validation time, and the shared compiler when it generates
packages. Affected parties are the authors of artifacts typed by this module.

## Dependencies

**Upstream**: the semantic-module contract and the semantic-core grammar.
**Downstream**: this module's own type declarations, which its maintainer writes.
