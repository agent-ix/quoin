---
id: NFR-020
title: "Declared external toolchain floors and absent-tool diagnostics"
type: NFR
quality_attribute: usability
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-076"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-077"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-080"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-083"
    type: "constrains"
---

# NFR-020: Declared external toolchain floors and absent-tool diagnostics

## Statement

Every external command the template or a rendered repository invokes SHALL have
a declared minimum version, and when it is absent or older than that version the
invoking command SHALL fail naming the command, the version required, and how to
install it.

## Scope

- Applies to: the renderer (`cookiecutter`), the schema toolchain (`node`, `npm`, `tsp` through `@typespec/compiler`), the Python toolchain (`poetry`), the validator (`quire validate`), and the Quire engine the rendered suite imports.
- Operational context: a maintainer's first render, and a clean runner with none of these installed.
- Not applied to: the shared reusable CI workflows, whose runner images declare their own toolchains.

## Rationale

FR-080 already does this for one tool — the Quire engine — and does it because
that tool's absence once turned every semantic row green. Every other external
command in the chain has the same failure shape and none of them had the same
treatment: a missing `tsp` makes the emit check pass with nothing emitted, a
missing `cookiecutter` makes the render gate report no variants rendered, and a
missing `quire` makes the spec gate report no documents validated. Each of those
is a green result for a check that did not run, which is the defect this
programme exists to end. Naming the version is not decoration: the emitter's
output shape is version-dependent, and a floor nobody states is a floor nobody
can meet on purpose.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| External commands with a declared minimum version | all | all | Test |
| External commands whose absence produces a named diagnostic | all | all | Test |
| Checks that report success with their tool absent | 0 | 0 | Test |

## Verification

Quoin's gate enumerates the external commands the template and a rendered
repository invoke, asserts each has a declared floor in one place, and, for each,
runs the invoking command in an environment where the tool is absent and asserts
the failure names the command, the floor, and the install step. A check that
passes with its tool absent fails this requirement.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-020-AC-1 | Every external command the template and a rendered repository invoke has a declared minimum version recorded in one file. | Test (TC-1463) |
| NFR-020-AC-2 | With the renderer absent, Quoin's render gate fails naming the renderer, the floor, and the install command. | Test (TC-1464) |
| NFR-020-AC-3 | With the schema toolchain absent, the emit and drift checks fail naming the toolchain and the install command, and report no skipped check. | Test (TC-1448) |
| NFR-020-AC-4 | With the validator absent, the rendered gate's validation leg fails naming the validator rather than reporting zero documents. | Test (TC-1465) |

## Dependencies

- **Upstream**: [FR-080](../functional/FR-080-generated-verification-suite.md), which establishes the pattern for the Quire engine
- **Downstream**: [FR-083](../functional/FR-083-template-render-self-tests.md)
