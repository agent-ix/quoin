---
id: FR-081
title: "Generated public repository and packaging baseline"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-075"
    type: "depends_on"
---

# FR-081: Generated public repository and packaging baseline

## Description

The rendered repository SHALL carry the licence, ownership, documentation, and
package surfaces a public AGPL-3.0-or-later module needs, with the module payload
published identically through its Python and npm surfaces and with no credential
committed.

## Rationale

A module repository is consumed two ways: Quoin installs the npm package and
Python tooling imports the wheel. Both must carry the same manifest, schemas, and
skeletons, or a consumer's view of the module depends on which surface it read.
The licence has to be one string everywhere — a `pyproject.toml` saying
AGPL-3.0-or-later beside a `package.json` saying MIT is a licensing defect, not a
typo, and it is the defect a copied cookiecutter default produces.

## Inputs

- `org`, `repo_name`, `package_name`, `author`, `email`, `version`, `license`

## Outputs

- `LICENSE`, `README.md`, `AGENTS.md`, `CLAUDE.md`, `.github/CODEOWNERS`, `.gitignore`, `.gitattributes`, `Makefile`
- `pyproject.toml` and `package.json` declaring the same licence and the same payload
- `.github/workflows/` release workflows that publish nothing until enabled

## Behavior

- The rendered repository SHALL carry the full AGPL-3.0-or-later licence text at `LICENSE`.
- Every rendered licence declaration — `pyproject.toml`, `package.json`, the README, and the generated package metadata — SHALL carry the same SPDX identifier.
- The rendered repository SHALL carry `.github/CODEOWNERS`, `AGENTS.md`, `CLAUDE.md`, a README, contribution guidance, security-reporting guidance, `.gitignore`, `.gitattributes`, and a `Makefile`.
- The rendered `pyproject.toml` and the rendered `package.json` SHALL each include the manifest, the emitted schemas, and the skeletons in the package they distribute.
- The rendered packaging SHALL exclude the schema toolchain from the distributed package, because the toolchain is a build input rather than module data.
- The rendered npm packaging SHALL stage the module payload so the published tarball root is the module root.
- The rendered npm packaging SHALL remove the staged copies after packing.
- The rendered repository SHALL declare its npm publication target as the public registry.
- The rendered repository SHALL contain no credential, no token, and no private-registry publication default.
- The rendered release workflows SHALL run only on a manual trigger.
- The rendered release workflows SHALL delegate to the organization's shared reusable workflows rather than carrying their own publish steps.
- The rendered README SHALL name the commands that install, validate, emit, and test the module.
- The rendered repository SHALL carry a document naming the steps to add the module to the Quoin default catalog and to the tracking project.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-081-CON-1 | The rendered repository SHALL declare no dependency by a local path reference, and no dependency with a version upper bound. | Packaging | Test (TC-1372) |
| FR-081-CON-2 | The rendered wheel and the rendered npm tarball SHALL carry the same manifest bytes, schema bytes, and skeleton bytes. | Consistency | Test (TC-1373) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-081-AC-1 | Every rendered variant carries the full AGPL-3.0-or-later text and one SPDX identifier across every declaration. | Test (TC-1374) |
| FR-081-AC-2 | Every rendered variant carries CODEOWNERS, AGENTS.md, CLAUDE.md, README, contribution and security guidance, .gitignore, .gitattributes, and a Makefile. | Test (TC-1375) |
| FR-081-AC-3 | Building the rendered Python package and packing the rendered npm package yield archives carrying the same manifest, schemas, and skeletons. | Test (TC-1373) |
| FR-081-AC-4 | No rendered file matches a credential, token, or private-registry publication pattern. | Test (TC-1376) |
| FR-081-AC-5 | The rendered npm publication target is the public registry and the package access is public. | Test (TC-1376) |
| FR-081-AC-6 | The rendered release workflows carry no publish step of their own and are triggered manually. | Test (TC-1377) |
| FR-081-AC-7 | The rendered catalog document names the Quoin catalog file and the tracking project to add the module to. | Test (TC-1378) |
| FR-081-AC-8 | No rendered dependency declaration uses a local path reference or a version upper bound. | Test (TC-1372) |

## Dependencies

- **Upstream**: [FR-075](./FR-075-semantic-package-exports-and-locks.md), [FR-076](./FR-076-semantic-module-template-variants.md)
- **Downstream**: [NFR-018](../non-functional/NFR-018-rendered-output-hygiene.md)
