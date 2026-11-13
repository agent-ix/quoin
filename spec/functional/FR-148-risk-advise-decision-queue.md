---
id: FR-148
title: "Risk advice decision queue: residual risks with no named decider, ordered by policy region"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-102"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-103"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-303"
    type: "depends_on"
---

# FR-148: Risk advice decision queue: residual risks with no named decider, ordered by policy region

## Description

Tracked by Linear PLAT-1651 and PLAT-1654. The `decision_queue` section of `quoin risk advise` (FR-145) lists the
residual risks nobody has decided, so a person can see what is waiting. A residual risk has no named decider
when its `decision` is `undecided` (`agent-ix/spec-objects-safety` FR-103). A post-control estimate with no
residual-risk record at all also has none and is listed separately. The queue orders work by the policy's own
regions; it proposes no decision.

## Behavior

- The command SHALL list one entry for every `residual_risk` in scope whose `decision` is `undecided`.
- Each entry SHALL carry the residual-risk id, its estimate id, its policy id, the estimate's entered severity and probability, and the policy region and `outside_acceptable` flag for that pair as FR-102 resolves them.
- The command SHALL read the policy from the residual risk's own `policy` reference.
- The command SHALL NOT use `--policy` for the queue.
- The command SHALL order entries by the position of their region in the referenced policy's region list, then by residual-risk id.
- If a policy's region list is ordered by the people who authored it, then the command SHALL use that order.
- The command SHALL NOT sort regions by name, by flag or by any severity it infers.
- If residual risks reference different policies, then the command SHALL group the entries by policy id in id order, each group ordered as above.
- If FR-102 resolves no region for an entry, then the entry SHALL carry `level-not-in-policy` or `level-not-assessed` as FR-146 does and no region.
- An entry with no region SHALL follow the entries of its group that have a region.
- The command SHALL list in a separate `no_record` group each post-control estimate that no `residual_risk` references, ordered by estimate id.
- The command SHALL NOT list a residual risk whose `decision` is `accepted` or `not_accepted`.
- The command SHALL NOT read a hazard's `status` as a decision.
- The command SHALL NOT offer a proposed decision, a recommended decision, a due date or a priority number.
- The Markdown rendering SHALL word each entry as waiting for a decision by a named person.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-148-AC-1 | Over residual risks in three decision states, only the `undecided` one is listed. | Test |
| FR-148-AC-2 | Three undecided residual risks whose regions sit at positions 2, 0 and 1 of the policy's region list are ordered 0, 1, 2, and two in one region are ordered by id. | Test |
| FR-148-AC-3 | A policy whose region list is reordered in the fixture reorders the queue accordingly, so order comes from the authored list and not from names or flags. | Test |
| FR-148-AC-4 | An undecided residual risk whose estimate has `probability: not_assessed` carries `level-not-assessed`, no region, and follows the entries with a region in its group. | Test |
| FR-148-AC-5 | Residual risks on two policies are grouped by policy id, each group in its own region order. | Test |
| FR-148-AC-6 | A post-control estimate with no referencing `residual_risk` is listed in `no_record`, and adding a record removes it from `no_record` and adds it to the queue. | Test |
| FR-148-AC-7 | A hazard at `status: accepted` with an `undecided` residual risk still lists the residual risk. | Test |
| FR-148-AC-8 | The JSON keys of an entry equal the set in this requirement exactly, with no decision, proposal, due date or priority key. | Test |
| FR-148-AC-9 | The Markdown for an entry in a region with `outside_acceptable: false` holds no wording that it can be accepted or is low priority. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md), [FR-146](./FR-146-risk-advise-policy-cells.md); `agent-ix/spec-objects-safety` FR-102, FR-103, FR-303.
- **Downstream**: none.
