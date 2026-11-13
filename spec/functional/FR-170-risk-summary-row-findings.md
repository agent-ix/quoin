---
id: FR-170
title: "Risk summary row findings: the chain's completeness findings placed against the row they name"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-169"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-155"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-147"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-105"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-301"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-302"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-303"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-204"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-305"
    type: "references"
---

# FR-170: Risk summary row findings: the chain's completeness findings placed against the row they name

## Description

Tracked by Linear PLAT-1576. The `findings` key of each row of `quoin risk-summary`
([FR-169](./FR-169-risk-summary-command.md)) holds the completeness findings of the chain that name that row, so a
reader sees a gap next to the hazard and situation it belongs to. This requirement coins no token and no rule. It
reuses the tokens and the rules of the safety module
([FR-301](ix://agent-ix/spec-objects-safety/FR-301), [FR-302](ix://agent-ix/spec-objects-safety/FR-302),
[FR-303](ix://agent-ix/spec-objects-safety/FR-303), [FR-105](ix://agent-ix/spec-objects-safety/FR-105)), as
`quoin risk advise` does for the evidence gaps ([FR-147](./FR-147-risk-advise-evidence-gaps.md)) and
`quoin risk discrepancies` does for decisions ([FR-155](./FR-155-risk-discrepancy-findings.md)). The command
computes the finding with the code those two commands use, so one fact has one implementation.

A finding states that two things people wrote disagree, or that a link or a record is missing. It does not say a
risk is acceptable, and the absence of findings in a row is not a decision.

## Behavior

- Each finding SHALL be an object with exactly the keys `finding` (the token) and `names` (the ids the token names, `[]` when it names none), as in FR-145.
- The command SHALL place each finding on every row whose situation, or whose hazard when the situation is `null`, the table below names.

| Token | Rule from | Placed on | `names` |
|---|---|---|---|
| `hazard-without-situation` | [FR-301](ix://agent-ix/spec-objects-safety/FR-301) | the row with `situation: null` of the hazard | the hazard id |
| `situation-without-hazard` | [FR-301](ix://agent-ix/spec-objects-safety/FR-301) | the row with `hazard: null` of the situation | the situation id |
| `situation-without-harm` | [FR-301](ix://agent-ix/spec-objects-safety/FR-301) | every row of the situation | the situation id |
| `control-without-requirement` | [FR-302](ix://agent-ix/spec-objects-safety/FR-302) | every row holding the control | the control id |
| `control-without-verification` | [FR-302](ix://agent-ix/spec-objects-safety/FR-302) | every row holding the control | the control id |
| `post-control-without-control` | [FR-204](ix://agent-ix/spec-objects-safety/FR-204) | every row of the situation the estimate estimates | the estimate id |
| `post-control-estimate-without-decision` | [FR-303](ix://agent-ix/spec-objects-safety/FR-303) | every row of the situation the estimate estimates | the estimate id |
| `residual-risk-undecided` | [FR-105](ix://agent-ix/spec-objects-safety/FR-105) | every row holding the residual risk | the residual-risk id |
| `decision-without-region` | [FR-105](ix://agent-ix/spec-objects-safety/FR-105) | every row holding the residual risk | the residual-risk id |
| `accepted-outside-acceptable-region` | [FR-105](ix://agent-ix/spec-objects-safety/FR-105) | every row holding the residual risk | the residual-risk id, then the covering benefit-risk ids |
| `benefit-risk-undecided` | [FR-105](ix://agent-ix/spec-objects-safety/FR-105) | every row holding the residual risk | the residual-risk id, then the covering benefit-risk ids |
| `benefit-risk-not-accepted` | [FR-105](ix://agent-ix/spec-objects-safety/FR-105) | every row holding the residual risk | the residual-risk id, then the covering benefit-risk ids |
| `not-accepted-inside-acceptable-region` | [FR-105](ix://agent-ix/spec-objects-safety/FR-105) | every row holding the residual risk | the residual-risk id |

- A control is verified, and implemented, exactly as FR-147 defines: an incoming `verifies` edge from any document, and an incoming `satisfies` edge from an `FR` or `NFR`. The command SHALL test that the edge exists and SHALL NOT read whether a verification passed.
- The residual-risk findings SHALL follow the rules of FR-155, with the region resolved from the residual risk's own `policy` reference as FR-155 does.
- A finding SHALL appear on a row only where the gap exists; a row whose link or record is present carries no such token.
- The command SHALL place `post-control-without-control` on every row of the situation of each post-control estimate that `assumes` no control, naming that estimate, whether or not another estimate of the situation assumes a control.
- The command SHALL emit no `estimate-without-situation` finding: an estimate with no situation, and the records that depend on it, are in no row and `quoin risk advise` lists them (FR-147).
- The command SHALL order the findings of a row by token in the order of the table above, then by the first element of `names`.
- A finding SHALL NOT carry a proposed remedy, a severity of the finding, a priority or a count.
- The command SHALL emit no finding for a hazard or a situation that has the link or record the rule asks for.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-170-AC-1 | The keys of every finding object equal `finding` and `names` exactly, over a fixture holding every token of the table. | Test |
| FR-170-AC-2 | A hazard with no situation yields its `situation: null` row with `hazard-without-situation` naming the hazard; adding a situation that `arises_from` it removes the finding. | Test |
| FR-170-AC-3 | A situation with no `arises_from` edge to a hazard yields its `hazard: null` row with `situation-without-hazard`; adding the edge moves it to the hazard's row and removes the finding. | Test |
| FR-170-AC-4 | A situation with no harm carries `situation-without-harm` on each of its rows with `harms: []`; adding a harm that `arises_from` it removes the finding. | Test |
| FR-170-AC-5 | A control with no `satisfies` edge from an `FR` or `NFR` carries `control-without-requirement` naming it, and one with no incoming `verifies` edge carries `control-without-verification` naming it; adding the edge removes the finding, and a link to the control in a requirement's body adds no edge. | Test |
| FR-170-AC-6 | A post-control estimate with no referencing residual risk carries `post-control-estimate-without-decision` naming the estimate; adding an `undecided` residual risk removes it and adds `residual-risk-undecided`. | Test |
| FR-170-AC-7 | An `accepted` residual risk in a region with `outside_acceptable: true` and no covering benefit-risk record carries `accepted-outside-acceptable-region`; an `accepted` covering record removes it, an `undecided` one yields `benefit-risk-undecided` naming that record, and a `not_accepted` one yields `benefit-risk-not-accepted`. | Test |
| FR-170-AC-8 | A `not_accepted` residual risk in a region with `outside_acceptable: false` carries `not-accepted-inside-acceptable-region`; an `accepted` one in that region carries no finding. | Test |
| FR-170-AC-9 | A decided residual risk whose estimate has `probability: not_assessed` carries `decision-without-region` and the entry for that estimate has `policy_region: null`. | Test |
| FR-170-AC-10 | For every fixture, the residual-risk findings of a row equal the `finding` tokens of `quoin risk discrepancies` for the same residual risks, and the control findings equal the control findings of `quoin risk advise` for the same controls. | Test |
| FR-170-AC-11 | A situation arising from two hazards carries its situation-level and control-level findings on both rows, and `hazard-without-situation` on neither, and a third fixture hazard with no situation carries it on its `situation: null` row alone. | Test |
| FR-170-AC-12 | Findings of a row are ordered by token in the table order, then by the first name; a complete chain has `findings: []` on every row, and the command states no acceptance for it. | Test |
| FR-170-AC-13 | No finding object holds a remedy, severity, priority or count key. | Test |
| FR-170-AC-14 | A control whose only verification is a fixture record stating a failed result still has the edge counted: the control carries no `control-without-verification`, and the output states no pass or fail. | Test |
| FR-170-AC-15 | A situation with two post-control estimates, one assuming a control and one assuming none, shows the control in `controls` and `post-control-without-control` naming the second estimate on every row of the situation; a pre-control estimate with no `assumes` edge yields none. | Test |
| FR-170-AC-16 | An estimate with no `estimates` edge yields no `estimate-without-situation` finding and appears in no row. | Test |

## Dependencies

- **Upstream**: [FR-169](./FR-169-risk-summary-command.md), [FR-145](./FR-145-risk-advise-command.md), [FR-147](./FR-147-risk-advise-evidence-gaps.md), [FR-155](./FR-155-risk-discrepancy-findings.md); [FR-105](ix://agent-ix/spec-objects-safety/FR-105), [FR-301](ix://agent-ix/spec-objects-safety/FR-301), [FR-302](ix://agent-ix/spec-objects-safety/FR-302), [FR-303](ix://agent-ix/spec-objects-safety/FR-303), [FR-204](ix://agent-ix/spec-objects-safety/FR-204).
- **Context only**: [FR-305](ix://agent-ix/spec-objects-safety/FR-305).
- **Downstream**: none.
