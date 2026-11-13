---
id: FR-155
title: "Risk discrepancy findings: the finding tokens, their shape and the severity flag"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-154"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-146"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-102"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-103"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-105"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-106"
    type: "depends_on"
---

# FR-155: Risk discrepancy findings: the finding tokens, their shape and the severity flag

## Description

Tracked by Linear PLAT-1570. Each finding of `quoin risk discrepancies` ([FR-154](./FR-154-risk-discrepancies-command.md))
is one of the tokens of `agent-ix/spec-objects-safety` FR-105, emitted in the `{ finding, names }` shape that
[FR-146](./FR-146-risk-advise-policy-cells.md) to [FR-148](./FR-148-risk-advise-decision-queue.md) use.

"Outside the acceptable region" is as `agent-ix/spec-objects-safety` FR-101 defines it: the region the policy
lists for the pair carries `outside_acceptable: true`. A benefit-risk record "covers" a residual risk when it
lists that risk (`agent-ix/spec-objects-safety` FR-106). The region is the one `agent-ix/spec-objects-safety`
FR-102 resolves, with the missing-region findings of FR-146. The `quality:ambiguous-term` lint on the Behavior lines that say "acceptable region" is a known false positive: the phrase repeats the defined term above and the tool applies no meaning of its own.

This section states one addition to FR-105: a post-control estimate that no residual risk references has no
decision at all, so the command reports it with the token `post-control-estimate-without-decision` of
FR-148, and does not wait for a residual-risk record to exist.

## Behavior

- If a residual risk is `accepted` or `not_accepted` and no region resolves for it, then the command SHALL report `decision-without-region`.
- If a residual risk is `accepted`, its region is outside the acceptable region and no benefit-risk record covers it, then the command SHALL report `accepted-outside-acceptable-region`.
- If a residual risk is `accepted`, its region is outside the acceptable region, covering records exist, none is `accepted` and at least one is `undecided`, then the command SHALL report `benefit-risk-undecided`.
- If a residual risk is `accepted`, its region is outside the acceptable region, covering records exist and all are `not_accepted`, then the command SHALL report `benefit-risk-not-accepted`.
- If a residual risk is `not_accepted` and its region is inside the acceptable region, then the command SHALL report `not-accepted-inside-acceptable-region`.
- If a residual risk is `undecided`, then the command SHALL report `residual-risk-undecided`, whatever its region.
- If a post-control estimate is referenced by no residual risk, then the command SHALL report `post-control-estimate-without-decision`.
- If a residual risk's decision agrees with the policy, then the command SHALL report no finding for it.
- The command SHALL read a decision only from the `decision` field of the record.
- The command SHALL NOT read a hazard's `status` as a decision.
- The command SHALL NOT propose a decision, a region or a remedy in a finding.
- The command SHALL order findings by the id of the first name in `names` (the residual-risk id, or the estimate id for `post-control-estimate-without-decision`), then by token.
- Each finding SHALL be an object with exactly the keys `finding` and `names`.
- `names` SHALL be an object with the keys below, each present, `null` or `[]` when it does not apply.

| Key | Content |
|---|---|
| `residual_risk` | the residual-risk id, or `null` for `post-control-estimate-without-decision` |
| `estimate` | the post-control estimate id |
| `decision` | the record's `decision` as written, or `null` |
| `region` | the policy's region name, or `null` |
| `policy` | the policy id from the residual risk's own `policy` reference, or `null` |
| `benefit_risk` | the ids of the covering benefit-risk records, `[]` if none |

- The Markdown rendering of each finding SHALL name the token, the residual risk, its recorded decision, the region and the policy.
- The Markdown rendering of a region SHALL read "the policy marks this cell as <region>".
- With `--strict` the command SHALL exit non-zero on any finding above (FR-154).
- The `--strict` flag SHALL NOT change which findings are reported or how they are worded.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-155-AC-1 | An `accepted` residual risk in a cell whose region has `outside_acceptable: true`, with no covering benefit-risk record, yields exactly one `accepted-outside-acceptable-region` finding whose `names` hold the residual-risk id, the estimate id, `accepted`, the region and the policy. | Test |
| FR-155-AC-2 | Adding an `accepted` benefit-risk record that covers that risk removes the finding. | Test |
| FR-155-AC-3 | A covering benefit-risk record with `decision: undecided` yields `benefit-risk-undecided` instead, with that record in `names.benefit_risk`. | Test |
| FR-155-AC-4 | A covering benefit-risk record that is `not_accepted` and is the only one yields `benefit-risk-not-accepted`; one `not_accepted` and one `accepted` covering record yield no benefit-risk finding. | Test |
| FR-155-AC-5 | A `not_accepted` residual risk in a cell whose region has `outside_acceptable: false` yields `not-accepted-inside-acceptable-region`. | Test |
| FR-155-AC-6 | An `undecided` residual risk yields `residual-risk-undecided` in a cell of either flag value, and with an all-acceptable policy the output still states no accepted state for it. | Test |
| FR-155-AC-7 | An `accepted` residual risk in a cell whose region has `outside_acceptable: false` yields no finding. | Test |
| FR-155-AC-8 | An `accepted` residual risk whose estimate has probability `not_assessed`, one whose severity is not in the policy, and a `not_accepted` risk in either state each yield `decision-without-region` and show no region. | Test |
| FR-155-AC-9 | A post-control estimate that no residual risk references yields `post-control-estimate-without-decision` with `residual_risk` `null`; adding a residual risk that references it replaces the finding with the findings this requirement states for that record, if any. | Test |
| FR-155-AC-10 | A hazard at `status: accepted` with an `undecided` residual risk yields `residual-risk-undecided`. | Test |
| FR-155-AC-11 | Every finding in JSON has exactly the keys `finding` and `names`, and every `names` has exactly the keys of the table above; findings are in the stated order. | Test |
| FR-155-AC-12 | The same fixture yields the same findings with and without `--strict`, and the Markdown of each finding contains "the policy marks this cell as" followed by the region when a region resolves. | Test |
| FR-155-AC-13 | Two residual risks on different policies each show their own policy id and their own region. | Test |

## Dependencies

- **Upstream**: [FR-154](./FR-154-risk-discrepancies-command.md), [FR-146](./FR-146-risk-advise-policy-cells.md); `agent-ix/spec-objects-safety` FR-101, FR-102, FR-103, FR-105, FR-106.
- **Context only**: [FR-148](./FR-148-risk-advise-decision-queue.md), which lists the same undecided records as a queue without comparing them to a region.
- **Downstream**: none.
