---
id: FR-155
title: "Risk discrepancy findings: the finding tokens, their shape and the strict flag"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-154"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-101"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-102"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-103"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-105"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-106"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-303"
    type: "depends_on"
---

# FR-155: Risk discrepancy findings: the finding tokens, their shape and the strict flag

## Description

Tracked by Linear PLAT-1570. Each finding of `quoin risk discrepancies` ([FR-154](./FR-154-risk-discrepancies-command.md))
is one of the tokens of [FR-105](ix://agent-ix/spec-objects-safety/FR-105), emitted as an object with the keys
`finding` and `refs`.

A region carries the flag `outside_acceptable` as [FR-101](ix://agent-ix/spec-objects-safety/FR-101) defines it. A
benefit-risk record covers a residual risk when it lists that risk
([FR-106](ix://agent-ix/spec-objects-safety/FR-106)). The region is the one
[FR-102](ix://agent-ix/spec-objects-safety/FR-102) resolves in the residual risk's own policy
([FR-103](ix://agent-ix/spec-objects-safety/FR-103)). When no region resolves, the finding
`decision-without-region` carries in `refs.region_missing` the reason, spelled as [FR-146](./FR-146-risk-advise-policy-cells.md) spells its tokens: `level-not-in-policy` when a level is not
in the policy's lists, `level-not-assessed` when a level is `unknown`, `not_assessed` or `not_applicable`,
`cell-not-in-policy` when the policy lists no region for the pair of levels, `policy-not-found` when the residual
risk names no policy or a policy that is not loaded, `policy-ambiguous` when it names more than one, or
`estimate-not-found` when the estimate it references is not loaded or was skipped for breaking its type's contract.

The token `post-control-estimate-without-decision` is owned by [FR-303](ix://agent-ix/spec-objects-safety/FR-303),
and `quoin risk advise` lists it in the `no_record` group of [FR-148](./FR-148-risk-advise-decision-queue.md). A
post-control estimate that no residual risk references has no decision at all, so this command reports it without
waiting for a residual-risk record.

## Behavior

- If a residual risk is `accepted` or `not_accepted` and no region resolves for it, then the command SHALL report `decision-without-region`.
- If a residual risk is `accepted`, its region has `outside_acceptable: true` and no benefit-risk record covers it, then the command SHALL report `accepted-outside-acceptable-region`.
- If a residual risk is `accepted`, its region has `outside_acceptable: true`, covering records exist, none is `accepted` and at least one is `undecided`, then the command SHALL report `benefit-risk-undecided`.
- If a residual risk is `accepted`, its region has `outside_acceptable: true`, covering records exist and all are `not_accepted`, then the command SHALL report `benefit-risk-not-accepted`.
- If a residual risk is `not_accepted` and its region has `outside_acceptable: false`, then the command SHALL report `not-accepted-inside-acceptable-region`.
- If a residual risk is `undecided`, then the command SHALL report `residual-risk-undecided`, whatever its region.
- If a post-control estimate is referenced by no residual risk, then the command SHALL report `post-control-estimate-without-decision`.
- If a residual risk's decision agrees with the policy, then the command SHALL report no finding for it.
- The command SHALL read a decision only from the `decision` field of the record.
- The command SHALL NOT read a hazard's `status` as a decision.
- The command SHALL NOT propose a decision, a region or a remedy in a finding.
- The command SHALL order findings by the estimate id in `refs.estimate`, then by residual-risk id, then by token.
- Each finding SHALL be an object with exactly the keys `finding` and `refs`.
- `refs` SHALL be an object with the keys below, each present, and `null` or `[]` when it does not apply.

| Key | Content |
|---|---|
| `residual_risk` | the residual-risk id, or `null` for `post-control-estimate-without-decision` |
| `estimate` | the post-control estimate id |
| `decision` | the record's `decision` as written, or `null` |
| `region` | the policy's region name, or `null` |
| `policy` | the policy id from the residual risk's own `policy` reference, or `null` |
| `benefit_risk` | the ids of the covering benefit-risk records, `[]` if none |
| `region_missing` | the reason above for `decision-without-region`, else `null` |

- The Markdown rendering of each finding SHALL name the token, the estimate id, the residual risk if any, its recorded decision, the region and the policy.
- The Markdown rendering of a region SHALL read "the policy marks this cell as <region>".
- The `--strict` flag SHALL NOT change which findings are reported or how they are worded.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-155-AC-1 | An `accepted` residual risk in a cell whose region has `outside_acceptable: true`, with no covering benefit-risk record, yields exactly one `accepted-outside-acceptable-region` finding whose `refs` hold the residual-risk id, the estimate id, `accepted`, the region and the policy. | Test |
| FR-155-AC-2 | Adding an `accepted` benefit-risk record that covers that risk removes the finding. | Test |
| FR-155-AC-3 | A covering benefit-risk record with `decision: undecided` yields `benefit-risk-undecided` instead, with that record in `refs.benefit_risk`. | Test |
| FR-155-AC-4 | A covering benefit-risk record that is `not_accepted` and is the only one yields `benefit-risk-not-accepted`; one `not_accepted` and one `accepted` covering record yield no benefit-risk finding. | Test |
| FR-155-AC-5 | A `not_accepted` residual risk in a cell whose region has `outside_acceptable: false` yields `not-accepted-inside-acceptable-region`. | Test |
| FR-155-AC-6 | An `undecided` residual risk yields `residual-risk-undecided` in a cell of either flag value, and with a policy whose regions all have `outside_acceptable: false` the output still states no accepted state for it. | Test |
| FR-155-AC-7 | An `accepted` residual risk in a cell whose region has `outside_acceptable: false` yields no finding. | Test |
| FR-155-AC-8 | An `accepted` residual risk whose estimate has probability `not_assessed`, and one whose severity is not in the policy, each yield `decision-without-region` with no region and `refs.region_missing` `level-not-assessed` and `level-not-in-policy` respectively; a `not_accepted` risk in either state does the same. | Test |
| FR-155-AC-9 | A post-control estimate that no residual risk references yields `post-control-estimate-without-decision` with `residual_risk` `null`, and its Markdown line names the estimate id; adding a residual risk that references it replaces the finding with the findings this requirement states for that record, if any. | Test |
| FR-155-AC-10 | A hazard at `status: accepted` with an `undecided` residual risk yields `residual-risk-undecided`. | Test |
| FR-155-AC-11 | Every finding in JSON has exactly the keys `finding` and `refs`, and every `refs` has exactly the keys of the table above; findings are in the stated order. | Test |
| FR-155-AC-12 | The same fixture yields the same findings with and without `--strict`, and the Markdown of each finding with a region contains "the policy marks this cell as" followed by the region. | Test |
| FR-155-AC-13 | Two residual risks on different policies each show their own policy id and their own region. | Test |
| FR-155-AC-14 | An `accepted` or `not_accepted` residual risk whose policy is not loaded, whose references name two policies, whose pair of levels the policy lists no region for, or whose estimate is not loaded, each yield `decision-without-region` with `refs.region_missing` `policy-not-found`, `policy-ambiguous`, `cell-not-in-policy` and `estimate-not-found` respectively. | Test |

## Dependencies

- **Upstream**: [FR-154](./FR-154-risk-discrepancies-command.md); [FR-101](ix://agent-ix/spec-objects-safety/FR-101), [FR-102](ix://agent-ix/spec-objects-safety/FR-102), [FR-103](ix://agent-ix/spec-objects-safety/FR-103), [FR-105](ix://agent-ix/spec-objects-safety/FR-105), [FR-106](ix://agent-ix/spec-objects-safety/FR-106), [FR-303](ix://agent-ix/spec-objects-safety/FR-303) in `agent-ix/spec-objects-safety`.
- **Context only**: [FR-148](./FR-148-risk-advise-decision-queue.md), which lists the same undecided records as a queue without comparing them to a region.
- **Downstream**: none.
