---
id: FR-148
title: "Risk advice decision queue: undecided residual-risk, benefit-risk and overall decisions, ordered by id"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-146"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-102"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-103"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-106"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-107"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-303"
    type: "depends_on"
---

# FR-148: Risk advice decision queue: undecided residual-risk, benefit-risk and overall decisions, ordered by id

## Description

Tracked by Linear PLAT-1651 and PLAT-1654. The `decision_queue` section of `quoin risk advise` (FR-145) lists the
decisions nobody has made, so a person can see what is waiting. A decision has no named decider when its
`decision` is `undecided` (`agent-ix/spec-objects-safety` FR-103, FR-106, FR-107), and the queue holds the
undecided `residual_risk`, `benefit_risk` and `overall_residual_risk_evaluation` records. A post-control
estimate with no residual-risk record at all also has no decision, and is listed separately under the token
`post-control-estimate-without-decision` of `agent-ix/spec-objects-safety` FR-303.

The queue is ordered by record kind and id only. The ticket scope asked for order by policy region; the policy
assigns its regions no priority (`agent-ix/spec-objects-safety` FR-101 defines only the `outside_acceptable`
flag), so ordering by region would be a prioritisation nobody authored. The region is shown as information.

## Behavior

- The command SHALL list one entry for every `residual_risk`, `benefit_risk` and `overall_residual_risk_evaluation` in scope whose `decision` is `undecided`.
- The command SHALL order entries by kind (`residual_risk`, then `benefit_risk`, then `overall_residual_risk_evaluation`), then by record id.
- For a `residual_risk` entry, the command SHALL carry the estimate id, the policy id from the record's own `policy` reference, the estimate's entered levels, and the region and `outside_acceptable` flag as `agent-ix/spec-objects-safety` FR-102 resolves them.
- If FR-102 resolves no region for a `residual_risk` entry, then the entry SHALL carry `level-not-in-policy` or `level-not-assessed` as FR-146 does and no region.
- For a `benefit_risk` entry, the command SHALL carry in `references` the ids of the residual risks it covers.
- For an `overall_residual_risk_evaluation` entry, the command SHALL carry in `references` the ids it considers.
- The command SHALL list in a separate `no_record` group each post-control estimate that no `residual_risk` references, with the finding `post-control-estimate-without-decision`, ordered by estimate id.
- The command SHALL NOT list a record whose `decision` is `accepted` or `not_accepted`.
- The command SHALL NOT read a hazard's `status` as a decision.
- The command SHALL NOT offer a proposed decision, a recommended decision, a due date or a priority number.
- The Markdown rendering of each entry SHALL read "waiting for a decision by a named person".

### Entry keys

Every key is present, `null` or `[]` when empty. The section is an object with the keys `queue` (the entries) and `no_record` (the group).

| Key | Content |
|---|---|
| `record` | the record id |
| `kind` | `residual_risk`, `benefit_risk` or `overall_residual_risk_evaluation` |
| `estimate` | the estimate id (`residual_risk` only), else `null` |
| `policy` | the policy id (`residual_risk` only), else `null` |
| `severity`, `probability` | as entered on the estimate (`residual_risk` only), else `null` |
| `region`, `outside_acceptable` | as the policy wrote them, or `null` |
| `references` | covered or considered ids (`benefit_risk`, `overall_residual_risk_evaluation`), else `[]` |
| `findings` | `{ finding, names }` objects for `level-not-in-policy` or `level-not-assessed` where they apply |

An element of `no_record` has exactly the keys `estimate` and `findings`, the latter one `{ finding, names }` object with `finding` `post-control-estimate-without-decision` and `names` `[]`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-148-AC-1 | Over a fixture of residual risks in the three decision states, only the `undecided` one is listed. | Test |
| FR-148-AC-2 | Undecided records of all three kinds are listed in the order residual risk, benefit risk, overall evaluation, and two of one kind are ordered by id. | Test |
| FR-148-AC-3 | Reordering the region list of the fixture policy leaves the queue order unchanged. | Test |
| FR-148-AC-4 | An undecided residual risk whose estimate has `probability: not_assessed` carries `level-not-assessed` and no region. | Test |
| FR-148-AC-5 | Each residual risk entry carries the policy id of its own `policy` reference, and two residual risks on different policies each show their own region. | Test |
| FR-148-AC-6 | A post-control estimate with no referencing `residual_risk` is listed in `no_record` with `post-control-estimate-without-decision`, and adding a record removes it from `no_record` and adds it to the queue. | Test |
| FR-148-AC-7 | A hazard at `status: accepted` with an `undecided` residual risk still lists the residual risk. | Test |
| FR-148-AC-8 | The keys of every entry and of each `no_record` element equal the key tables, with no decision, proposal, due date or priority key. | Test |
| FR-148-AC-9 | The Markdown of every entry reads "waiting for a decision by a named person" and holds no wording that a risk can be accepted or is low priority. | Test |
| FR-148-AC-10 | An undecided benefit-risk record lists the residual risks it covers in `references`, and an undecided overall evaluation lists the ids it considers. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md), [FR-146](./FR-146-risk-advise-policy-cells.md); [FR-102](ix://agent-ix/spec-objects-safety/FR-102), [FR-103](ix://agent-ix/spec-objects-safety/FR-103), [FR-106](ix://agent-ix/spec-objects-safety/FR-106), [FR-107](ix://agent-ix/spec-objects-safety/FR-107), [FR-303](ix://agent-ix/spec-objects-safety/FR-303).
- **Downstream**: none.
