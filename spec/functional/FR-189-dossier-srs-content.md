---
id: FR-189
title: "The dossier SRS lists every requirement in scope with its criteria and recorded criticality"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "extends"
---

# FR-189: The dossier SRS lists every requirement in scope with its criteria and recorded criticality

## Description

The software requirements specification (`--doc srs`, document id `DOS-SRS`)
SHALL list every requirement document in scope (StR, FR, NFR) in id order with
its id, title, statement, acceptance criteria (id and text) and criticality as
recorded.

## Inputs

- The requirement documents of the `requirement-document` and `nfr-document` archetypes in the scope, and the `requirements[]` entries of the quire-rs coverage matrix.
- The optional `criticality` key of each requirement (quire-rs FR-082), read from frontmatter.

## Outputs

- `dossier/DOS-SRS.md`, one level-2 section per requirement, then `Gaps`.

## Behavior

- The SRS SHALL list requirements in id order.
- The SRS SHALL show each criticality exactly as recorded.
- If a requirement has no `criticality`, the SRS SHALL show `unassigned` for it.
- If a requirement has no `criticality`, the SRS SHALL record a `criticality_unassigned` gap naming it.
- If a requirement has zero acceptance criteria, the SRS SHALL record an `unbacked_criterion` gap naming it.
- If the scope holds no requirement, the SRS SHALL record a `no_requirements` gap.
- The SRS SHALL NOT infer, default or compute a criticality level.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-189-AC-1 | A fixture with three FRs, the third with no `criticality`, renders three sections in id order, and the third shows `unassigned` with a `criticality_unassigned` gap | Test |
| FR-189-AC-2 | A requirement with zero criteria produces an `unbacked_criterion` gap naming it | Test |
| FR-189-AC-3 | Absent criticality never renders a level: the section holds no level word for that requirement | Test |
| FR-189-AC-4 | An empty scope renders no requirement section, a `no_requirements` gap and status `incomplete` | Test |
| FR-189-AC-5 | Every criterion id and text of the fixture appears under its requirement | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-dossier-command.md); quire-rs FR-082 (criticality).
- **Downstream**: none.
