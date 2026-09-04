---
id: NFR-018
title: "Rendered semantic-module output is public-ready and free of generation residue"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-076"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-081"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-083"
    type: "constrains"
---

# NFR-018: Rendered semantic-module output is public-ready and free of generation residue

## Statement

Every rendered semantic-module repository SHALL carry zero unresolved template
tokens, zero placeholder organizations, zero absolute paths from the rendering
machine, zero credentials, zero private-registry publication defaults, and
exactly one licence identifier, so that the rendered tree is publishable as it
stands.

## Scope

- Applies to: every file of every rendered variant, at every depth, including dotfiles and workflow files.
- Operational context: the first commit of a new public module repository, before any hand editing.

## Rationale

Each residue class has a distinct consequence and none is cosmetic. An unresolved
token or placeholder organization produces a repository that names somebody
else's project. An absolute path binds the repository to the machine that
generated it. A credential in a public repository is a disclosure. A
private-registry publication default sends a package intended to be public to an
internal index, where its absence is discovered by the consumer. Two licence
identifiers make the licence a question rather than a grant.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Unresolved template tokens per rendered variant | 0 | 0 | Test |
| Placeholder organizations per rendered variant | 0 | 0 | Test |
| Absolute rendering-machine paths per rendered variant | 0 | 0 | Test |
| Credential or token matches per rendered variant | 0 | 0 | Test |
| Private-registry publication defaults per rendered variant | 0 | 0 | Test |
| Distinct licence identifiers per rendered variant | 1 | 1 | Test |

## Verification

Quoin's gate renders each variant into a temporary directory and scans every file
for each residue class, reporting the file and the match. The scan is the same
one FR-083 runs, and its negative cases are exercised by injecting one instance
of each class into the template and asserting the gate fails naming it.

The patterns are not left to the reader. Each residue class is defined by a
declared pattern in the template's conformance contract, beside the required
surfaces, so that "a credential" and "a private-registry publication default"
are comparisons against a reviewable list rather than a judgement call made in a
test body. The private-registry class is defined over publication configuration
— the npm `publishConfig` registry, any Poetry package source, any workflow
publish target — and not over every mention of a private host: the rendered
`dev-quire` provisioning command names one, deliberately and with the tracking
issue beside it, because no index the repository may depend on serves the engine
(`agent-ix/quire-rs#392`). That one exemption is declared in the contract and
asserted to be the only one.

## Dependencies

- **Upstream**: [FR-076](../functional/FR-076-semantic-module-template-variants.md), [FR-081](../functional/FR-081-generated-public-repository-baseline.md)
- **Downstream**: [FR-083](../functional/FR-083-template-render-self-tests.md)
