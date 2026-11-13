---
id: FR-147
title: "Risk advice evidence gaps: controls not implemented or verified, and open anomalies on the hazard or the implementing requirement"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-136"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-202"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-203"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-204"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-205"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-301"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-302"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-305"
    type: "references"
---

# FR-147: Risk advice evidence gaps: controls not implemented or verified, and open anomalies on the hazard or the implementing requirement

## Description

Tracked by Linear PLAT-1651 and PLAT-1653. The `evidence_gaps` section of `quoin risk advise` (FR-145) shows,
for each post-control risk estimate, where the things its estimate rests on are weakly evidenced, so a person can
see how much weight the estimate bears. It reports where a link or a record is missing. It does not say whether
a control works or whether a verification passed, because no record carries that (`agent-ix/spec-objects-safety`
FR-205).

### The same tokens and the same rule as the safety module

This section coins no finding token and no second rule. It reuses the tokens of `agent-ix/spec-objects-safety`
and their rules, so it states the same facts as `quoin risk-summary` (FR-305) about the same control:

| Token | Source | Meaning here |
|---|---|---|
| `post-control-without-control` | FR-204 | the post-control estimate assumes no control |
| `control-without-requirement` | FR-302 | no `FR` or `NFR` has a `satisfies` edge to the control |
| `control-without-verification` | FR-302 | the control is not verified |
| `estimate-without-situation` | FR-203 | the estimate has no situation |
| `situation-without-hazard` | FR-301 | the situation arises from no hazard |

A control is **verified** exactly as `agent-ix/spec-objects-safety` FR-302 defines it: it has an incoming
`verifies` edge from any document. The report tests that the edge exists and never reads whether the verification passed.

A control "of an estimate" is a control the post-control estimate `assumes`. The "hazard of an estimate" is the
hazard its situation `arises_from` (`agent-ix/spec-objects-safety` FR-202). An anomaly is "open" when the
`status` in its record is not `closed`.

## Behavior

- The command SHALL list one entry for every post-control `risk_estimate` in scope, ordered by estimate id.
- For each control of the estimate, the entry SHALL carry `control-without-requirement` under the rule above.
- For each control of the estimate, the entry SHALL carry `control-without-verification` under the rule above.
- If the estimate assumes no control, then the entry SHALL carry `post-control-without-control`.
- If the estimate has no situation, then the entry SHALL carry `estimate-without-situation`.
- If the estimate's situation arises from no hazard, then the entry SHALL carry `situation-without-hazard`.
- The entry SHALL list each open anomaly whose assessment names a hazard of the estimate.
- The entry SHALL list each open anomaly whose assessment names an `FR` or `NFR` that `satisfies` a control of the estimate, or an acceptance criterion of that requirement.
- Each listed anomaly SHALL carry `anomaly`, `kind` and `impact` as `quoin anomaly refs` reports them, and `names`, the id the assessment names.
- The command SHALL NOT read, set or infer an `effective`, `verified`, `passed` or `status`-derived value of a control.
- The command SHALL NOT read a test or evidence outcome.
- The Markdown rendering SHALL word a missing verification as "no verification is linked".
- The Markdown rendering SHALL NOT use the words "ineffective" or "unsafe".
- The report SHALL NOT state that an estimate with no gap is sound.
- The command SHALL NOT rank estimates by number of gaps.
- The command SHALL emit no count-based score.

### Entry keys

Every key is present, `null` or `[]` when empty.

| Key | Content |
|---|---|
| `estimate` | the post-control estimate id |
| `situation` | the estimate's situation id, or `null` |
| `hazards` | ids of the hazards of the estimate |
| `controls` | each control of the estimate: `control`, `requirements` (ids), `verifications` (ids of verifying documents), `findings` |
| `anomalies` | each open anomaly listed above, as `{ anomaly, kind, impact, names }` |
| `findings` | the estimate-level tokens above that name this entry |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-147-AC-1 | A post-control estimate assuming one control with no `satisfies` edge yields `control-without-requirement` on that control; adding the edge removes it. | Test |
| FR-147-AC-2 | A control with no incoming `verifies` edge yields `control-without-verification`; adding a `verifies` edge from a test document removes it. | Test |
| FR-147-AC-3 | A control whose only verifying test document is recorded as failing is not flagged `control-without-verification`, and the entry carries no key or wording about the outcome. | Test |
| FR-147-AC-4 | Over a fixture, the tokens of each control and estimate equal those that the FR-302, FR-204, FR-203 and FR-301 rules yield over the same fixture records. | Test |
| FR-147-AC-5 | A hazard named by one open and one `closed` anomaly lists only the open one, as `{ anomaly, kind, impact, names }` with `names` the hazard id. | Test |
| FR-147-AC-6 | An open anomaly naming the `FR` that satisfies a control of the estimate, and one naming an acceptance criterion of that `FR`, are both listed with `names` the id they name; a closed one is not. | Test |
| FR-147-AC-7 | A post-control estimate assuming no control yields `post-control-without-control`, one with no situation yields `estimate-without-situation`, and one whose situation has no hazard yields `situation-without-hazard`. | Test |
| FR-147-AC-8 | A pre-control estimate yields no entry. | Test |
| FR-147-AC-9 | The Markdown line for a missing verification reads "no verification is linked" and the output holds neither "ineffective" nor "unsafe". | Test |
| FR-147-AC-10 | The keys of every entry and of each `controls` and `anomalies` element equal the entry key table, with no count, score or rank key, and two entries with different gap numbers are ordered by estimate id. | Test |
| FR-147-AC-11 | A fixture with no gap lists its control, carries no finding token and no statement of soundness. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md), [FR-136](./FR-136-anomaly-record-mapping.md); [FR-202](ix://agent-ix/spec-objects-safety/FR-202), [FR-203](ix://agent-ix/spec-objects-safety/FR-203), [FR-204](ix://agent-ix/spec-objects-safety/FR-204), [FR-205](ix://agent-ix/spec-objects-safety/FR-205), [FR-301](ix://agent-ix/spec-objects-safety/FR-301), [FR-302](ix://agent-ix/spec-objects-safety/FR-302).
- **Context only**: [FR-305](ix://agent-ix/spec-objects-safety/FR-305).
- **Downstream**: none.
