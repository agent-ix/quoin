---
id: FR-147
title: "Risk advice evidence gaps: controls not implemented or verified, and open anomalies on the hazard"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-136"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-202"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-204"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-205"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-302"
    type: "depends_on"
---

# FR-147: Risk advice evidence gaps: controls not implemented or verified, and open anomalies on the hazard

## Description

Tracked by Linear PLAT-1651 and PLAT-1653. The `evidence_gaps` section of `quoin risk advise` (FR-145) shows,
for each post-control risk estimate, which of the things its estimate rests on are weakly evidenced, so a person
can see how much weight the estimate bears. It reports where evidence is missing; it does not say whether a
control works, because no record carries that (`agent-ix/spec-objects-safety` FR-205).

A control "of an estimate" is a control the post-control estimate `assumes`. The "hazard of an estimate" is the
hazard its situation `arises_from` (`agent-ix/spec-objects-safety` FR-202). An anomaly is "open" when its
`status` is not `closed`.

## Behavior

- The command SHALL list one entry for every post-control `risk_estimate` in scope, ordered by estimate id.
- Each entry SHALL carry the estimate id, the ids of its situation and hazards, its controls, and its open anomalies, each possibly empty.
- For each control of the estimate, the entry SHALL carry `control-not-implemented` when no `FR` or `NFR` has a `satisfies` edge to the control.
- For each control of the estimate, the entry SHALL carry `control-not-verified` when no document has a `verifies` edge to the control and no `FR` or `NFR` implementing the control has an acceptance criterion bound by retained evidence.
- The entry SHALL list, for each hazard of the estimate, the open anomalies whose assessment names that hazard, as `{ anomaly, impact, status }` as `quoin anomaly refs` reports them.
- If a post-control estimate assumes no control, then the entry SHALL carry `no-control-assumed`.
- If a post-control estimate has no situation or no hazard, then the entry SHALL carry `no-hazard-found`.
- The command SHALL test that a link exists.
- The command SHALL NOT read, set or infer an `effective`, `verified`, `passed` or `status`-derived value of a control.
- The Markdown rendering SHALL word a missing verification as "no verification is linked", never as "ineffective" or "unsafe".
- An entry with no gap SHALL list its controls and anomalies and carry no gap token.
- The report SHALL NOT state that an estimate with no gap is sound.
- The command SHALL NOT rank estimates by number of gaps.
- The command SHALL emit no count-based score.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-147-AC-1 | A post-control estimate assuming one control with no `satisfies` edge yields `control-not-implemented` naming the control; adding the edge removes it. | Test |
| FR-147-AC-2 | A control with no `verifies` edge and no retained evidence yields `control-not-verified`; adding a `verifies` edge, or retained evidence on its implementing requirement, removes it. | Test |
| FR-147-AC-3 | A hazard named by one open and one `closed` anomaly lists only the open one, with its impact and status. | Test |
| FR-147-AC-4 | A post-control estimate assuming no control yields `no-control-assumed`, and one whose situation has no hazard yields `no-hazard-found`. | Test |
| FR-147-AC-5 | A pre-control estimate yields no entry. | Test |
| FR-147-AC-6 | Over a control whose verifying test is recorded as failing, the entry carries no gap token for verification and no key or wording about effectiveness, because only link existence is tested. | Test |
| FR-147-AC-7 | The Markdown line for a missing verification reads "no verification is linked" and the output holds neither "ineffective" nor "unsafe". | Test |
| FR-147-AC-8 | The JSON of an entry holds no count, score or rank key, and two entries with different gap numbers are ordered by estimate id. | Test |
| FR-147-AC-9 | A fixture with no gap lists its control and carries no gap token and no statement of soundness. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md), [FR-136](./FR-136-anomaly-record-mapping.md); `agent-ix/spec-objects-safety` FR-202, FR-204, FR-205, FR-302.
- **Downstream**: none.
