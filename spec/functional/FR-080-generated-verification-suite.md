---
id: FR-080
title: "Generated verification suite treats the engine as a hard dependency"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-079"
    type: "depends_on"
---

# FR-080: Generated verification suite treats the engine as a hard dependency

## Description

The rendered repository SHALL carry a verification suite that fails, naming the
provisioning command, when the Quire engine exposing `extract_semantic` is
absent, never skipping a semantic check for that reason, so that a green run
means the checks ran.

## Rationale

The Quire engine exposing `extract_semantic` is a declared dev dependency,
resolved from the `internal-pypi` Poetry source by `poetry install`. Declaring
it does not make it reliably present: a checkout where `poetry install` has not
run, or has run against a stale lock, still lacks the engine, and the obvious
handling of that gap — `pytest.importorskip` — turns every semantic row green
in exactly the environment where none of them ran. A skipped row is not
coverage, and a clean CI runner that has not yet installed is precisely the
environment that would skip.

## Inputs

- The rendered `<package>/manifest.yaml`, skeletons, schemas, and fixtures
- The Quire engine, resolved as a dev dependency by `poetry install`

## Outputs

- A passing suite when the engine is present and the module conforms
- A failing suite naming `poetry install` when the engine is absent

## Behavior

- The rendered test configuration SHALL import the Quire engine.
- If the engine cannot be imported, then the rendered test configuration SHALL fail the test naming `poetry install`.
- If the engine imports but does not expose `extract_semantic`, then the rendered test configuration SHALL fail naming the missing capability.
- The rendered suite SHALL NOT call a skip for a missing engine, a missing grammar package, or a missing schema toolchain.
- The rendered repository SHALL declare the engine as a dev dependency of its package metadata, sourced from the `internal-pypi` Poetry source.
- The rendered repository SHALL carry a `semantic-install` command that installs the npm-resolved schema toolchain (`@typespec/compiler`, `@agent-ix/semantic-core`) via `npm ci`.
- The rendered suite SHALL assert the rendered `semantic` block, the emitted schemas, the skeleton mappings, the negative fixtures, and the legacy-form fixture.
- When the grammar package is not installed, the rendered suite SHALL fail naming the install command rather than skipping the schema checks.
- The rendered suite SHALL treat every warning as an error, so that a deprecation is a failure rather than scrollback.
- The rendered repository's gate SHALL run spec validation, lint, the schema drift check, and the rendered suite, failing when any one of them fails.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-080-CON-1 | The rendered suite SHALL skip no test for a missing tool, recording an expected engine gap as a test marked to fail strictly — one that fails the run if it unexpectedly passes — beside a control test proving the failure has the declared cause. | Coverage | Test (TC-1427) |
| FR-080-CON-2 | The rendered `pyproject.toml` SHALL declare the engine as a dev dependency sourced from the `internal-pypi` Poetry source. | Contract | Test (TC-1430) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-080-AC-3 | A run of the rendered suite reports zero skipped tests. | Test (TC-1427) |
| FR-080-AC-4 | The rendered package metadata declares the engine as a dev dependency sourced from `internal-pypi`. | Test (TC-1430) |
| FR-080-AC-5 | The rendered gate runs spec validation, lint, the schema drift check, and the suite, and a failing validation leg fails the gate before lint and the suite run. | Test (TC-1431) |
| FR-080-AC-6 | The rendered test configuration turns warnings into errors. | Test (TC-1430) |

## Dependencies

- **Upstream**: [FR-078](./FR-078-generated-manifest-semantic-block.md), [FR-079](./FR-079-generated-skeletons-and-fixtures.md)
- **Downstream**: [FR-083](./FR-083-template-render-self-tests.md)
