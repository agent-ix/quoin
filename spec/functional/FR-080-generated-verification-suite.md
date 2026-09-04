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

No index a repository may commit against carries the wheel exposing
`extract_semantic` today (`agent-ix/quire-rs#392`), so the engine cannot be a
declared dependency and has to be provisioned out of band. The obvious handling —
`pytest.importorskip` — turns every semantic row green in exactly the environment
where none of them ran. A skipped row is not coverage, and a clean CI runner is
precisely the environment that would skip.

## Inputs

- The rendered `<package>/manifest.yaml`, skeletons, schemas, and fixtures
- The Quire engine, provisioned by the rendered `dev-quire` command

## Outputs

- A passing suite when the engine is present and the module conforms
- A failing suite naming `make dev-quire` and the tracking issue when the engine is absent

## Behavior

- The rendered test configuration SHALL import the Quire engine.
- If the engine cannot be imported, then the rendered test configuration SHALL fail the test naming the provisioning command and the tracking issue.
- If the engine imports but does not expose `extract_semantic`, then the rendered test configuration SHALL fail naming the missing capability.
- The rendered suite SHALL NOT call a skip for a missing engine, a missing grammar package, or a missing schema toolchain.
- The rendered repository SHALL carry a `dev-quire` command that installs the engine from the index that serves it.
- The rendered repository SHALL NOT declare the engine as a dependency of its package metadata, because no index it is permitted to depend on serves the required version.
- The rendered suite SHALL assert the rendered `semantic` block, the emitted schemas, the manifest digests, the skeleton mappings, the negative fixtures, and the legacy-form fixture.
- When the grammar package is not installed, the rendered suite SHALL fail naming the install command rather than skipping the schema checks.
- The rendered suite SHALL treat every warning as an error, so that a deprecation is a failure rather than scrollback.
- The rendered suite SHALL compare the installed engine's version with the engine floor the rendered repository declares.
- If the installed engine is older than the declared floor, then the rendered suite SHALL fail naming both versions, rather than reporting a capability gap as a module defect.
- The rendered repository's gate SHALL run spec validation, lint, the schema drift check, and the rendered suite, failing when any one of them fails.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-080-CON-1 | The rendered suite SHALL skip no test for a missing tool, recording an expected engine gap as a test marked to fail strictly — one that fails the run if it unexpectedly passes — beside a control test proving the failure has the declared cause. | Coverage | Test (TC-1427) |
| FR-080-CON-2 | The rendered `dev-quire` command SHALL name the tracking issue for the missing published wheel. | Traceability | Test (TC-1430) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-080-AC-1 | With the engine absent, the rendered suite fails and its message names the provisioning command and the tracking issue. | Test (TC-1428) |
| FR-080-AC-2 | With the engine present but without `extract_semantic`, the rendered suite fails naming the missing capability. | Test (TC-1429) |
| FR-080-AC-3 | A run of the rendered suite reports zero skipped tests. | Test (TC-1427) |
| FR-080-AC-4 | The rendered package metadata declares no dependency on the engine. | Test (TC-1430) |
| FR-080-AC-5 | The rendered gate runs spec validation, lint, the schema drift check, and the suite, and fails when any one fails. | Test (TC-1431) |
| FR-080-AC-6 | The rendered test configuration turns warnings into errors. | Test (TC-1430) |
| FR-080-AC-7 | With `@agent-ix/semantic-core` not installed, the rendered suite fails naming the install command, and reports no skipped schema check. | Test (TC-1450) |
| FR-080-AC-8 | With an engine older than the declared floor installed, the rendered suite fails naming the installed version and the floor. | Test (TC-1460) |

## Dependencies

- **Upstream**: [FR-078](./FR-078-generated-manifest-semantic-block.md), [FR-079](./FR-079-generated-skeletons-and-fixtures.md), `agent-ix/quire-rs#392`
- **Downstream**: [FR-083](./FR-083-template-render-self-tests.md)
