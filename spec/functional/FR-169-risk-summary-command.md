---
id: FR-169
title: "quoin risk-summary: a read-only report of the hazard-to-decision chain, one row per hazard and situation"
type: FR
relationships:
  - target: "ix://agent-ix/spec-objects-safety/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/spec-objects-safety/FR-305"
    type: "implements"
  - target: "ix://agent-ix/spec-objects-safety/FR-104"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-102"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-202"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-203"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-204"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-205"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-146"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-170"
    type: "references"
---

# FR-169: quoin risk-summary: a read-only report of the hazard-to-decision chain, one row per hazard and situation

## Description

Tracked by Linear PLAT-1576. `quoin risk-summary <scope>` is the command that
[FR-305](ix://agent-ix/spec-objects-safety/FR-305) defines in the safety module. It reads the risk records below
`<scope>` and prints the whole chain for each hazard: the hazardous situation, the harms, the pre-control and
post-control estimates, the controls with their implementing requirements and verifications, and the residual-risk
decisions. It is a view of what people wrote, as `quoin risk advise` ([FR-145](./FR-145-risk-advise-command.md))
and the SOUP report of [FR-144](./FR-144-soup-report.md) are: it persists nothing, adds no record type and changes
no record.

### What this command adds to the two existing risk reports

`quoin risk advise` is organised per estimate and per undecided record (FR-145). `quoin risk discrepancies`
([FR-154](./FR-154-risk-discrepancies-command.md)) lists only places where a decision disagrees with the policy.
Neither shows a hazard with its situation, harms and controls in one place, and neither lists a hazard or a
situation that has no link. This command is the chain view, and it carries exactly what those two cannot:

- one row for every hazard and situation pairing, including a hazard with no situation and a situation with no hazard;
- the harms of each situation;
- the controls the post-control estimates assume, with the ids of the requirements that implement them and of the verifications that verify them;
- the residual-risk decision and its decider as recorded;
- the completeness findings of the chain, placed against the row they name ([FR-170](./FR-170-risk-summary-row-findings.md)).

It does not repeat what those reports carry. The policy cell, the cell's `outside_acceptable` flag and the reason a
region is missing stay in `quoin risk advise` ([FR-146](./FR-146-risk-advise-policy-cells.md)); the queue of
undecided decisions stays in `quoin risk advise` ([FR-148](./FR-148-risk-advise-decision-queue.md)). This report
shows only the region name, resolved by the rules of FR-146, so that where both reports show a value they show the
same value.

### Region lookup is shared, not repeated

The `policy_region` of an estimate entry is the region name that the FR-146 policy-source rules and the
[FR-102](ix://agent-ix/spec-objects-safety/FR-102) lookup give for that estimate. The command SHALL use the same
implementation as `quoin risk advise`. This requirement states no second lookup rule.

## Behavior

### Surface and reading

- When a user runs `quoin risk-summary <scope> [--format markdown|json] [--module DIR]...`, the command SHALL print the report described below. `--format` defaults to `markdown`.
- The command SHALL read the records below `<scope>` and check them against the safety module exactly as FR-145 does: the quire engine's reader, the `body_extraction` of each type's declaring module, taken from each `--module` root or, with none given, from the installed modules.
- The command SHALL read these record kinds: `hazard`, `harm`, `hazardous_situation`, `risk_estimate`, `control`, `residual_risk`, `benefit_risk`, the `acceptability_policy` documents they reference, and the `FR`, `NFR`, `IT` and `TC` documents that link to a control.
- The command SHALL take the edges of a record from the frontmatter `relationships` only; a link in a table cell or elsewhere in the body is no edge.
- The command SHALL skip a `risk_estimate` whose `phase` is not `pre_control` or `post_control`, or that breaks the contract of its type, as FR-145 does, and SHALL be silent about it.

### Exit codes and diagnostics

- The command SHALL exit zero whenever it renders, whatever the rows contain.
- If `<scope>` does not exist or is unreadable, a module does not load, a type it must check is declared by no loaded module, or a `residual_risk`, `benefit_risk` or `acceptability_policy` breaks the contract of its type, then the command SHALL exit 2, name the path or the record and the reason, and print no report. Each failure SHALL use the diagnostic code that `quoin risk advise` uses for the same failure, and the "type declared by no loaded module" message SHALL name `--module DIR`.
- If the reader cannot load a file below `<scope>`, then the command SHALL name the file and the reason on stderr and still render, and the file is absent from the report.
- If `--format` has any value but `markdown` or `json`, then the command SHALL exit 2 as a usage error and print no report.
- If `<scope>` contains no hazard and no hazardous situation, then the command SHALL print the attribution and an empty `rows` list, and exit zero.

### Output contract

- `--format json` SHALL write one JSON object with exactly the keys `attribution` (a string) and `rows` (an array).
- `--format markdown` SHALL write the attribution statement, then one table whose columns are the row keys below, in the order of the table, then nothing else.
- The `attribution` string SHALL state that regions come from the project's policy and that decisions come from the named person.
- The command SHALL write one row for each pairing of a hazard and a hazardous situation that `arises_from` it ([FR-202](ix://agent-ix/spec-objects-safety/FR-202)). A hazard with no such situation has one row with `situation: null`. A situation with no `arises_from` edge to a hazard has one row with `hazard: null`. A situation that arises from two hazards appears in two rows with the same situation content.
- Every row SHALL have exactly the keys below, each present, `null` or `[]` when empty.

| Key | Content |
|---|---|
| `hazard` | the hazard id, or `null` |
| `situation` | the situation id, or `null` |
| `harms` | ids of the harms that `arises_from` the situation, ascending |
| `pre_control` | estimate entries for the `pre_control` estimates that `estimates` the situation |
| `controls` | control entries for each control that a `post_control` estimate of the situation `assumes` |
| `post_control` | estimate entries for the `post_control` estimates that `estimates` the situation |
| `residual_risks` | residual-risk entries for each `residual_risk` whose `estimate` edge names a `post_control` estimate of the situation |
| `findings` | finding objects of [FR-170](./FR-170-risk-summary-row-findings.md) that name this row |

- Every estimate entry SHALL have exactly the keys `estimate`, `severity`, `probability` and `policy_region`. `severity` and `probability` are as entered, including the markers `unknown`, `not_assessed` and `not_applicable`. `policy_region` is the policy's own region name for those levels, or `null` where the FR-146 rules yield no region; it is a key of the entry and not of the row.
- Every control entry SHALL have exactly the keys `control`, `requirements` and `verifications`. `requirements` holds the ids of the `FR` and `NFR` documents with a `satisfies` edge to the control. `verifications` holds the ids of the documents with a `verifies` edge to the control, whatever their type. Both are ascending, `[]` when none.
- Every residual-risk entry SHALL have exactly the keys `residual_risk`, `decision` and `decided_by`. `decision` is `undecided`, `accepted` or `not_accepted` as the record wrote it. `decided_by` is as the record wrote it, `null` where the record has none.
- When two or more estimates of a phase exist for one situation, the command SHALL list all of them and SHALL mark none as in force (the choice among them belongs to the people who authored them, [FR-203](ix://agent-ix/spec-objects-safety/FR-203)).
- A control assumed by several post-control estimates of one situation SHALL appear once in that row.

### Ordering

- The command SHALL order rows by hazard id, then situation id, with `null` after every id in each position: the rows of a hazard with no situation follow that hazard's rows, and the rows with `hazard: null` come last, ordered by situation id.
- The command SHALL order the entries of every list by the entry's id, ascending in byte order, and the `findings` as FR-170 states.

### No verdict, no score, no stored value

These rules bind this command itself. NFR-031 binds `quoin risk advise` only and is not widened.

- The command SHALL NOT write any file or any record, and SHALL store no score, region, finding or verdict.
- The command SHALL emit no key named or typed as a score, weight, total, rank, verdict, recommendation, proposal, priority or count of risk.
- The command SHALL NOT combine, average or compare levels, and SHALL NOT compare a pre-control entry with a post-control entry.
- The command SHALL print a level, a marker, a decision and a decider as entered, and SHALL NOT substitute a scale value or a blank for an absent value or a marker. In Markdown an absent value reads `null`.
- The command SHALL print the stem "accept" only in the attribution, in a finding token, and in a value copied from a record.
- In Markdown, a region SHALL read "the policy marks this cell as <region>", and a missing region SHALL read "the policy lists no region".
- The command SHALL NOT read a hazard's `status` as a decision ([FR-103](ix://agent-ix/spec-objects-safety/FR-103)).
- For identical inputs, each format SHALL render byte-identical output.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-169-AC-1 | Over a complete fixture chain, `--format json` parses as one object whose keys equal `attribution` and `rows` exactly, and has one row per hazard and situation pairing. | Test |
| FR-169-AC-2 | The keys of every row equal the row key table exactly; the keys of every estimate entry equal `estimate`, `severity`, `probability`, `policy_region` exactly; of every control entry `control`, `requirements`, `verifications` exactly; of every residual-risk entry `residual_risk`, `decision`, `decided_by` exactly. | Test |
| FR-169-AC-3 | A hazard with no situation yields one row with `situation: null` and empty lists; a situation with no hazard yields one row with `hazard: null`; a situation arising from two hazards appears in two rows with equal situation content. | Test |
| FR-169-AC-4 | A fixture estimate with `probability: not_assessed` shows `not_assessed` and `policy_region: null`, while another estimate in the same row keeps its own region name. | Test |
| FR-169-AC-5 | A fixture residual risk with `decision: undecided` shows `undecided` and `decided_by: null`, and no row shows `accepted` that a record did not write; a hazard with `status: accepted` changes no residual-risk entry. | Test |
| FR-169-AC-6 | Two post-control estimates on one situation both appear in `post_control`, and two pre-control estimates in `pre_control`, and no key marks any as in force. | Test |
| FR-169-AC-7 | A control assumed by two post-control estimates of one situation appears once in `controls`; a requirement with a `satisfies` edge and a test with a `verifies` edge to it appear by id in `requirements` and `verifications`; a link to the control in the body of a requirement appears in neither. | Test |
| FR-169-AC-8 | Rows are ordered by hazard id then situation id, with the `situation: null` rows of a hazard after its other rows and the `hazard: null` rows last; entries within every list are ordered by id. | Test |
| FR-169-AC-9 | For every estimate of a fixture, `policy_region` equals the `region` that `quoin risk advise` shows for that estimate over the same fixture, `null` exactly where `region` is `null`. | Test |
| FR-169-AC-10 | After the command runs, with each format, over a fixture repository, no file is created, changed or removed. | Test |
| FR-169-AC-11 | Two runs over the same inputs give byte-identical Markdown and byte-identical JSON. | Test |
| FR-169-AC-12 | No key anywhere in the JSON is named or typed as a score, weight, total, rank, verdict, recommendation, proposal, priority or count of risk, and no value is numeric except where copied from a record. | Test |
| FR-169-AC-13 | In the Markdown over every fixture the stem "accept" appears only in the attribution, in a finding token, and in values copied from a record; each resolved region reads "the policy marks this cell as" followed by the region name; an absent value reads `null`. | Test |
| FR-169-AC-14 | The attribution string names the policy as the source of regions and the named person as the source of decisions. | Test |
| FR-169-AC-15 | Over an empty scope the command exits zero and prints an empty `rows` list with the attribution. | Test |
| FR-169-AC-16 | A nonexistent scope, a `residual_risk` with an unknown decision word, an `acceptability_policy` with an unfilled placeholder level, and a type declared by no loaded module each exit 2 naming the path or record, print no report, and use the diagnostic code of the same failure in `quoin risk advise`; the last message names `--module DIR`. | Test |
| FR-169-AC-17 | A `risk_estimate` with a `phase` outside the two members, and one with no judgement table, are skipped silently and the command exits zero. | Test |
| FR-169-AC-18 | `--format xml` exits 2 and prints no report; omitting `--format` renders Markdown. | Test |
| FR-169-AC-19 | Over a snapshot fixture corpus holding one complete chain and one chain with each gap of FR-170, the Markdown and the JSON each equal their checked-in snapshots byte for byte. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md), [FR-146](./FR-146-risk-advise-policy-cells.md); [FR-102](ix://agent-ix/spec-objects-safety/FR-102), [FR-104](ix://agent-ix/spec-objects-safety/FR-104), [FR-202](ix://agent-ix/spec-objects-safety/FR-202), [FR-203](ix://agent-ix/spec-objects-safety/FR-203), [FR-204](ix://agent-ix/spec-objects-safety/FR-204), [FR-205](ix://agent-ix/spec-objects-safety/FR-205).
- **Context only**: [FR-144](./FR-144-soup-report.md), [FR-148](./FR-148-risk-advise-decision-queue.md), [FR-154](./FR-154-risk-discrepancies-command.md); [FR-305](ix://agent-ix/spec-objects-safety/FR-305).
- **Downstream**: [FR-170](./FR-170-risk-summary-row-findings.md).
