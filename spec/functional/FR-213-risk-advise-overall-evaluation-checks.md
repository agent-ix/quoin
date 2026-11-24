---
id: FR-213
title: "Risk advice overall-evaluation checks: residual risks not considered, and an evaluation decided over an open risk"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-148"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-103"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-107"
    type: "depends_on"
---

# FR-213: Risk advice overall-evaluation checks: residual risks not considered, and an evaluation decided over an open risk

## Description

Tracked by Linear PLAT-1809. `agent-ix/spec-objects-safety` FR-107 specifies two findings over an
`overall_residual_risk_evaluation`: a residual risk the evaluation never considered, and an evaluation that is
`accepted` while a considered residual risk is not. The `overall_evaluation_checks` section of `quoin risk advise`
([FR-145](./FR-145-risk-advise-command.md)) reports both, using FR-107's tokens and rules, so a reader sees the
disagreement without the tool deciding anything. The bundle being read is the product (FR-107): every
`residual_risk` document in scope belongs to it.

A finding states that two things people wrote disagree. It does not say the evaluation is right or wrong, and
it never derives an evaluation. When the scope holds no evaluation document the section is empty: the tool
creates none and reports no accepted state. The dossier's `no_overall_evaluation` gap
([FR-199](./FR-199-dossier-risk-management-content.md)) states that absence; this section does not repeat it.

## Behavior

- The command SHALL list one entry per `overall_residual_risk_evaluation` in scope, ordered by record id, in the `overall_evaluation_checks` section, whatever its `decision`.
- Each entry SHALL have exactly the keys `evaluation` (the record id), `product` (its `Product` cell, `null` when empty), `decision` (its decision word as written) and `findings`.
- The command SHALL take the considered list of an evaluation from its frontmatter `references` edges to `residual_risk` and `benefit_risk` documents; a link in the body is no edge.
- If a `residual_risk` in scope is not in the considered list of an evaluation, then the entry SHALL carry one `residual-risk-not-considered` finding naming every such residual risk in ascending order.
- If an evaluation's decision is `accepted` and a considered `residual_risk` has decision `undecided` or `not_accepted`, then the entry SHALL carry one `evaluation-accepted-with-open-risk` finding naming those residual risks in ascending order.
- The command SHALL NOT raise `evaluation-accepted-with-open-risk` for an evaluation whose decision is `undecided` or `not_accepted`, and SHALL NOT read a benefit-risk record's decision for it.
- The command SHALL order the findings of an entry `residual-risk-not-considered` first.
- The command SHALL NOT derive an evaluation decision, create an evaluation, or state an accepted state from the considered records.
- If the scope holds no evaluation, then the section SHALL be `[]`.
- A finding SHALL carry no remedy, severity, priority or count.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-213-AC-1 | Over a fixture with two residual risks and an evaluation that references one, the entry carries one `residual-risk-not-considered` naming the other; referencing both removes it. | Test |
| FR-213-AC-2 | An `accepted` evaluation over a considered `not_accepted` residual risk carries `evaluation-accepted-with-open-risk` naming it. | Test |
| FR-213-AC-3 | An `accepted` evaluation over a considered `undecided` residual risk carries `evaluation-accepted-with-open-risk` naming it; over only `accepted` residual risks it carries none. | Test |
| FR-213-AC-4 | Given only `accepted` residual risks and no evaluation document, `overall_evaluation_checks` is `[]`, the command creates no file, and no output reports an accepted state of the product. | Test |
| FR-213-AC-5 | An `undecided` or `not_accepted` evaluation over an open considered residual risk carries no `evaluation-accepted-with-open-risk`. | Test |
| FR-213-AC-6 | The keys of every entry equal `evaluation`, `product`, `decision`, `findings` exactly, and no entry or finding holds a score, class, acceptability, remedy, severity or priority key. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md), [FR-148](./FR-148-risk-advise-decision-queue.md); [FR-103](ix://agent-ix/spec-objects-safety/FR-103), [FR-107](ix://agent-ix/spec-objects-safety/FR-107).
- **Downstream**: [FR-199](./FR-199-dossier-risk-management-content.md).
