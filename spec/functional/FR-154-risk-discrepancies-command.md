---
id: FR-154
title: "quoin risk discrepancies: a read-only report of where a recorded risk decision disagrees with the authored policy"
type: FR
relationships:
  - target: "ix://agent-ix/spec-objects-safety/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/spec-objects-safety/FR-104"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-105"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-145"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-146"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/NFR-031"
    type: "references"
---

# FR-154: quoin risk discrepancies: a read-only report of where a recorded risk decision disagrees with the authored policy

## Description

Tracked by Linear PLAT-1570. `quoin risk discrepancies <scope>` reads the residual-risk, benefit-risk and
post-control estimate records below `<scope>` and the policies they reference, and reports each place where a
person's recorded decision disagrees with the region the policy lists for the estimate's levels. The finding
rules are those of `agent-ix/spec-objects-safety` FR-105; [FR-155](./FR-155-risk-discrepancy-findings.md) states
how this command emits them.

It is a separate command from `quoin risk advise` ([FR-145](./FR-145-risk-advise-command.md)), not a fourth
section of it. FR-145 fixes the report to exactly the keys `attribution`, `policy_cells`, `evidence_gaps` and
`decision_queue`, and fixes the exit status to zero whenever it renders. A discrepancy report needs a findings
list and, for a gate, a non-zero exit under a flag; adding either to `risk advise` would break two of FR-145's
criteria. Region lookup is not repeated here: the command takes the region and the `outside_acceptable` flag of
each residual risk from the same lookup as [FR-146](./FR-146-risk-advise-policy-cells.md), so the two commands
cannot show different regions for one estimate.

The command reports; it does not decide. Acceptance is a human decision (`agent-ix/spec-objects-safety` FR-104).
A finding says that two things a person wrote disagree. The absence of findings is not an acceptance.

## Behavior

- When a user runs `quoin risk discrepancies <scope> [--format markdown|json] [--strict]`, the command SHALL print the `attribution` and the `findings` list described in FR-155.
- The command SHALL read only the `residual_risk`, `benefit_risk` and `risk_estimate` records below `<scope>` and the `acceptability_policy` documents they reference.
- The command SHALL NOT write any file or any record.
- The command SHALL store no score, region, finding or verdict.
- The command SHALL emit no acceptability verdict for any risk, and no list of risks that agree with their policy.
- The command SHALL render Markdown by default and one JSON object with `--format json`.
- The JSON object SHALL have exactly the keys `attribution` and `findings`.
- The `attribution` string SHALL state that regions come from the project's policy and that decisions come from the named person.
- If `<scope>` contains no residual risk and no post-control estimate, then the command SHALL print an empty `findings` list and the attribution, and exit zero.
- Without `--strict`, the command SHALL exit zero whenever it renders, whatever the findings are.
- With `--strict`, the command SHALL exit non-zero when it reports one or more findings.
- With `--strict` and no finding, the command SHALL exit zero.
- If `<scope>` does not exist or a record in it cannot be parsed, then the command SHALL exit non-zero with an error naming the path or the record, as the other `quoin` read commands do.
- For identical inputs, each format SHALL render byte-identical output.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-154-AC-1 | Over a fixture with one finding, `--format json` parses as one object whose keys equal `attribution` and `findings` exactly. | Test |
| FR-154-AC-2 | After the command runs over a fixture repository, with and without `--strict`, no file is created, changed or removed. | Test |
| FR-154-AC-3 | Two runs over the same inputs give byte-identical Markdown and byte-identical JSON. | Test |
| FR-154-AC-4 | Over an empty scope the command exits zero and prints an empty `findings` list with the attribution. | Test |
| FR-154-AC-5 | Over a fixture that yields findings, the command exits zero without `--strict` and non-zero with `--strict`; over a fixture that yields none, it exits zero with `--strict`. | Test |
| FR-154-AC-6 | The attribution string names the policy as the source of regions and the named person as the source of decisions. | Test |
| FR-154-AC-7 | The JSON and Markdown hold no key or wording that states a risk accepted, acceptable, compliant or passing, no score, and no entry for a risk that has no finding. | Test |
| FR-154-AC-8 | For a residual risk, the region in a finding equals the region `quoin risk advise` shows for its estimate over the same fixture. | Test |
| FR-154-AC-9 | A nonexistent `<scope>` exits non-zero naming the path, and a malformed residual-risk record exits non-zero naming the record. | Test |

## Dependencies

- **Upstream**: `agent-ix/spec-objects-safety` FR-104, FR-105; [FR-146](./FR-146-risk-advise-policy-cells.md).
- **Context only**: [FR-145](./FR-145-risk-advise-command.md), [NFR-031](../non-functional/NFR-031-risk-advice-states-and-never-decides.md), whose wording rules this report follows.
- **Downstream**: [FR-155](./FR-155-risk-discrepancy-findings.md).
