---
id: FR-082
title: "Generated governance tree validates as rendered"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-015"
    type: "depends_on"
---

# FR-082: Generated governance tree validates as rendered

## Description

The rendered repository SHALL carry a `spec/` tree that passes `quire validate`
structurally as rendered, including a Test Matrix whose every cell is drawn from
the archetype's vocabularies and whose rows state honestly what the rendered
repository does not yet cover.

## Rationale

A rendered spec tree that fails validation makes the first act in a new
repository a repair, and teaches the maintainer that the gate is noise. The
Test Matrix is the sharper case: `⚠️` was admitted by an older contract and
classed by the traceability model as nothing, so every row carrying it was exempt
from the status-lie check by construction, and a matrix that shipped it reached
`main` in a sibling repository as recently as this campaign. The rendered matrix
must therefore be valid by construction and must express incompleteness with the
marker the archetype admits, carrying the reason.

## Inputs

- The rendered repository identity and exported types

## Outputs

- `spec/spec.md`, `spec/index.md`, `spec/log.md`, `spec/tests.md`
- `spec/stakeholder/`, `spec/usecase/`, `spec/functional/`, `spec/non-functional/`, each with an index

## Behavior

- The rendered `spec/` tree SHALL validate structurally under `quire validate` with no error, as rendered and before any editing.
- The rendered `spec/` tree SHALL carry a master-requirements root, a stakeholder requirement, a user story, and the functional requirements that describe the rendered module's own contract.
- The rendered `spec/` tree SHALL carry an index and a log for each folder that the reserved archetypes require one for.
- The rendered Test Matrix SHALL draw every `Status` cell from the markers the `TestMatrix` archetype admits.
- The rendered Test Matrix SHALL NOT use the retired `⚠️` marker in any cell.
- Where a rendered row is not yet covered, the rendered Test Matrix SHALL mark it `🚧` followed by the reason, rather than marking it complete or omitting it.
- The rendered Test Matrix SHALL trace every row to an acceptance criterion of the rendered spec rather than to another row.
- The rendered repository's gate SHALL run `quire validate` over the rendered `spec/` tree, so a later edit that breaks it fails the gate.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-082-CON-1 | The rendered Test Matrix `Status` vocabulary SHALL be the one `spec_artifacts_process/manifest.yaml` declares, not a superset. | Contract | Test (TC-1380) |
| FR-082-CON-2 | The rendered spec tree SHALL describe the rendered module, carrying no requirement copied from an existing module repository. | Independence | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-082-AC-1 | `quire validate` over each rendered variant's `spec/**/*.md` exits zero with no error diagnostic. | Test (TC-1379) |
| FR-082-AC-2 | No rendered Test Matrix cell carries `⚠️`, and every `Status` cell matches the archetype's pattern. | Test (TC-1380) |
| FR-082-AC-3 | Every rendered Test Matrix row that is not covered carries `🚧` and a reason. | Test (TC-1381) |
| FR-082-AC-4 | Every rendered Test Matrix row traces to an acceptance criterion that exists in the rendered spec. | Test (TC-1382) |
| FR-082-AC-5 | Each rendered variant carries the master-requirements root, the stakeholder, usecase, functional, and non-functional folders, and their indexes. | Test (TC-1383) |

## Dependencies

- **Upstream**: [FR-015](./FR-015-emit-quire-validate-command.md), [FR-076](./FR-076-semantic-module-template-variants.md)
- **Downstream**: [FR-083](./FR-083-template-render-self-tests.md)
