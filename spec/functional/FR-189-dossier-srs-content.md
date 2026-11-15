---
id: FR-189
title: "The dossier SRS lists every requirement in scope with its criteria and recorded criticality"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "extends"
---

# FR-189: The dossier SRS lists every requirement in scope with its criteria and recorded criticality

## Description

When `quoin dossier generate` selects the SRS (`--doc srs`, document id `DOS-SRS`), the SRS SHALL list every requirement document in scope (StR, FR, NFR) in dossier order (FR-188) with its id, title, statement, criteria (id and text) and criticality as recorded.

## Inputs

- The `requirements` source: the requirement documents in the scope.
- For an FR or NFR, the criteria are the rows of its `Acceptance Criteria` table, read as the `requirements[]` entries of the quire-rs coverage matrix give them (criterion `id` and `statement`).
- For an StR, the criteria are the rows of its `Validation Criteria` table.
- The statement is the text of the `Description` section of an FR, the `Statement` section of an NFR and the `Stakeholder Need` section of an StR.
- The optional `criticality` key of each requirement (quire-rs FR-082), read from frontmatter.

## Outputs

- `dossier/DOS-SRS.md`, one level-2 section per requirement, then `Gaps`.

## Behavior

- The SRS SHALL list requirements in dossier order.
- The SRS SHALL show each criticality exactly as recorded.
- If a requirement has no `criticality`, the SRS SHALL show `unassigned` for it.
- If a requirement has no `criticality`, the SRS SHALL record a `criticality_unassigned` gap naming it.
- If a requirement has zero criteria in its criteria table, the SRS SHALL record a `no_acceptance_criteria` gap naming it.
- The SRS SHALL list the Validation Criteria of an StR as its criteria.
- If the scope holds no requirement, the SRS SHALL record a `no_requirements` gap.
- The SRS SHALL NOT infer, default or compute a criticality level.
- The SRS SHALL NOT record an `unbacked_criterion` gap, which is reserved for a criterion without a test tag (FR-191).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-189-AC-1 | A fixture with three FRs, the third with no `criticality`, renders three sections in dossier order, and the third shows `unassigned` with a `criticality_unassigned` gap | Test |
| FR-189-AC-2 | A requirement with zero criteria produces a `no_acceptance_criteria` gap naming it and no `unbacked_criterion` gap | Test |
| FR-189-AC-3 | Absent criticality never renders a level: the section holds no level word for that requirement | Test |
| FR-189-AC-4 | An empty scope renders no requirement section, a `no_requirements` gap and status `incomplete` | Test |
| FR-189-AC-5 | Every criterion id and text of the fixture appears under its requirement | Test |
| FR-189-AC-6 | A fixture StR with a Validation Criteria table lists each validation criterion id and text under the StR and records no `no_acceptance_criteria` gap for it | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-dossier-command.md); quire-rs FR-082 (criticality).
- **Downstream**: none.
