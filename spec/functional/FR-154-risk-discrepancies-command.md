---
id: FR-154
title: "quoin risk discrepancies: a read-only report of where a recorded risk decision disagrees with the authored policy"
type: FR
relationships:
  - target: "ix://agent-ix/spec-objects-safety/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/spec-objects-safety/FR-101"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-104"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-105"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-145"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-146"
    type: "depends_on"
---

# FR-154: quoin risk discrepancies: a read-only report of where a recorded risk decision disagrees with the authored policy

## Description

Tracked by Linear PLAT-1570. `quoin risk discrepancies <scope>` reads the residual-risk, benefit-risk and
post-control estimate records below `<scope>` and the policies they reference, and reports each place where a
person's recorded decision disagrees with the region the policy lists for the estimate's levels. The finding
rules are those of [FR-105](ix://agent-ix/spec-objects-safety/FR-105); [FR-155](./FR-155-risk-discrepancy-findings.md)
states how this command emits them.

It is a separate command from `quoin risk advise` ([FR-145](./FR-145-risk-advise-command.md)), not a fourth
section of it. FR-145 fixes the report to exactly the keys `attribution`, `policy_cells`, `evidence_gaps` and
`decision_queue`, and fixes the exit status to zero whenever it renders. A discrepancy report needs a findings
list and, for a gate, a non-zero exit under a flag; adding either to `risk advise` would break two of FR-145's
criteria.

### Region and policy

Each residual risk names its own policy ([FR-103](ix://agent-ix/spec-objects-safety/FR-103)). This command looks the
region up in that policy, for the estimate's two levels, as [FR-102](ix://agent-ix/spec-objects-safety/FR-102) does.
`quoin risk advise` ([FR-146](./FR-146-risk-advise-policy-cells.md)) takes the policy of an estimate from the
residual risks that reference it. The two agree whenever the estimate has exactly one policy: both then show
the same region. When two residual risks on different policies reference one estimate, FR-146 shows
`policy-ambiguous` and no region for that estimate, while this command shows each residual risk against its own
policy and region.

The command reports; it does not decide. Acceptance of a residual risk is a human decision
([FR-104](ix://agent-ix/spec-objects-safety/FR-104)). A finding says that two things a person wrote disagree.
The absence of findings is not an acceptance.

## Behavior

- When a user runs `quoin risk discrepancies <scope> [--format markdown|json] [--strict]`, the command SHALL print the `attribution` and the `findings` list described in FR-155.
- The command SHALL read only the `residual_risk`, `benefit_risk` and `risk_estimate` records below `<scope>` and the `acceptability_policy` documents they reference.
- The command SHALL NOT write any file or any record.
- The command SHALL store no score, region, finding or verdict.
- The command SHALL emit no verdict that a risk is accepted, compliant or passing, and no entry for a risk that has no finding.
- The command SHALL print the stem "accept" only in the attribution, in a finding token, and in a value copied from a record.
- The command SHALL emit no key named or typed as a score, weight, total, rank, verdict, recommendation, proposal or priority.
- The command SHALL render Markdown by default and one JSON object with `--format json`.
- The JSON object SHALL have exactly the keys `attribution` and `findings`.
- The `attribution` string SHALL state that regions come from the project's policy and that decisions come from the named person.
- If `<scope>` contains no residual risk and no post-control estimate, then the command SHALL print an empty `findings` list and the attribution, and exit zero.
- Where `--strict` is not given, the command SHALL exit zero whenever it renders.
- Where `--strict` is given and the command reports one or more findings, the command SHALL exit 1.
- Where `--strict` is given and the command reports no finding, the command SHALL exit zero.
- If `<scope>` does not exist or a record in it cannot be parsed, then the command SHALL exit 2 with an error naming the path or the record, whether or not `--strict` is given.
- For identical inputs, each format SHALL render byte-identical output.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-154-AC-1 | Over a fixture with one finding, `--format json` parses as one object whose keys equal `attribution` and `findings` exactly. | Test |
| FR-154-AC-2 | After the command runs over a fixture repository, with and without `--strict`, no file is created, changed or removed. | Test |
| FR-154-AC-3 | Two runs over the same inputs give byte-identical Markdown and byte-identical JSON. | Test |
| FR-154-AC-4 | Over an empty scope the command exits zero and prints an empty `findings` list with the attribution. | Test |
| FR-154-AC-5 | Over a fixture that yields findings, the command exits zero without `--strict` and 1 with `--strict`; over a fixture that yields none, it exits zero with `--strict`. | Test |
| FR-154-AC-6 | The attribution string names the policy as the source of regions and the named person as the source of decisions. | Test |
| FR-154-AC-7 | In the Markdown over every fixture, the stem "accept" appears only in the attribution, in a finding token, and in values copied from a record; the output states no risk accepted, compliant or passing and has no entry for a risk without a finding; the JSON holds no score, weight, total, rank, verdict, recommendation, proposal or priority key. | Test |
| FR-154-AC-8 | For a residual risk whose post-control estimate is referenced only by residual risks on one policy, the region in its finding equals the region `quoin risk advise` shows for that estimate over the same fixture. | Test |
| FR-154-AC-9 | A nonexistent `<scope>` and a malformed residual-risk record each exit 2 naming the path or the record, with and without `--strict`. | Test |
| FR-154-AC-10 | Over a fixture in which two residual risks on different policies reference one estimate, `quoin risk advise` shows `policy-ambiguous` and no region for it, and this command reports each residual risk against its own policy and region. | Test |

## Dependencies

- **Upstream**: [FR-101](ix://agent-ix/spec-objects-safety/FR-101), [FR-104](ix://agent-ix/spec-objects-safety/FR-104), [FR-105](ix://agent-ix/spec-objects-safety/FR-105) in `agent-ix/spec-objects-safety`; [FR-146](./FR-146-risk-advise-policy-cells.md).
- **Context only**: [FR-145](./FR-145-risk-advise-command.md).
- **Downstream**: [FR-155](./FR-155-risk-discrepancy-findings.md).
