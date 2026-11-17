---
id: FR-204
title: "Report formalization state per requirement and criterion"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-184"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-094"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-153"
    type: "references"
---
# FR-204: Report formalization state per requirement and criterion

## Description

Quoin SHALL report exactly one formalization state for every requirement and
acceptance criterion in the selected corpus: `none`, `draft`, `approved`, or
`analysed`.

## Inputs

- The selected specification corpus and its requirement and acceptance-criterion identifiers.
- The QSL `formalizes` relations for those identifiers.
- The approval state derived under FR-184.
- The latest judging disposition for selected declarations, when one exists, classified under FR-153.

## Outputs

- `quoin formal-rules coverage [--repo] [--json]` reports one item for each requirement and acceptance criterion.
- With `--json`, the command emits `quoin.formalization-coverage/v1`, with `items[]` containing `id`, `kind` (`requirement` or `criterion`), `document`, optional `criticality`, `state`, `declarations[]`, and optional `analysis_disposition`.
- Each declaration in `declarations[]` SHALL include its approval state; when a declaration is `draft`, it also includes the draft reason returned by the approval-state derivation.
- The JSON `totals` groups item counts by state for each document and each criticality. An absent or null criticality is counted under `unassigned`.

## Behavior

- A requirement or criterion with no declaration linked by `formalizes` SHALL have state `none`.
- A requirement or criterion with one or more formalizing declarations SHALL have state `draft` if any such declaration has approval state `draft`, even when another declaration is approved.
- For each draft declaration, Quoin SHALL preserve its derived approval reason in the coverage output.
- A requirement or criterion SHALL have state `approved` when it has formalizing declarations, every such declaration is approved, and no latest selected judging run has a `qualified-record` disposition for those declarations.
- A requirement or criterion SHALL have state `analysed` only when every formalizing declaration is approved and the latest judging run that selected them has the FR-153 disposition `qualified-record`.
- Quoin SHALL report any latest analysis disposition alongside `approved`.
- Quoin SHALL treat no disposition other than `qualified-record` as `analysed`.
- When no selected judging run has a disposition, Quoin SHALL report `unevaluated` as the analysis disposition alongside `approved`.
- Quoin SHALL derive a requirement's state independently of the states of its acceptance criteria.
- Quoin SHALL derive every item's state from its formalizes relations, [FR-184](./FR-184-analysis-act-producer-definition.md) approval state, and [FR-153](./FR-153-analysis-result-counting-by-disposition.md) analysis disposition.
- Quoin SHALL NOT store a second formalization-state record.
- Totals SHALL equal the counts of the listed items in their respective document, criticality, and state groups.
- The output SHALL NOT contain an aggregate score or percentage.

## Dependencies

- **Upstream:** [FR-184](./FR-184-analysis-act-producer-definition.md) defines the derived approval state; [FR-153](./FR-153-analysis-result-counting-by-disposition.md) defines the `qualified-record` disposition; quire-rs [FR-094](ix://agent-ix/quire-rs/FR-094) defines the QSL `formalizes` relation, with amendments [FR-051](ix://agent-ix/quire-rs/FR-051), [FR-062](ix://agent-ix/quire-rs/FR-062), and [FR-077](ix://agent-ix/quire-rs/FR-077).
- **Downstream:** [FR-205](./FR-205-formalization-as-evidence-kind.md) presents these states as evidence.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-204-AC-1 | Given corpus items with no formalizing declarations, draft declarations, only approved declarations without a qualified analysis, and approved declarations with a latest `qualified-record` disposition, each item is listed exactly once with state `none`, `draft`, `approved`, and `analysed` respectively. | Test |
| FR-204-AC-2 | Given one approved and one draft declaration formalizing the same item, the item is `draft`, including when the approved declaration has a qualified analysis. | Test |
| FR-204-AC-3 | Given a draft declaration with a derived approval reason, the coverage output retains that reason with the declaration. | Test |
| FR-204-AC-4 | Given approved declarations and a latest analysis disposition of `unevaluated`, `failed`, `errored`, `absent`, `stale`, or `qualification-rejected`, the item remains `approved` and the disposition is shown; none of these dispositions yields `analysed`. | Test |
| FR-204-AC-5 | Given an approved declaration and no selected judging run, the item is `approved` and its analysis disposition is `unevaluated`. | Test |
| FR-204-AC-6 | Given a requirement and its acceptance criteria with different formalization evidence, each criterion's state is derived independently and does not change the requirement's state. | Test |
| FR-204-AC-7 | Given a corpus with a null or absent criticality, the item's count appears in `unassigned`; for every document and criticality group, each state total equals the number of listed items in that group and state. | Test |
| FR-204-AC-8 | Given any coverage output, the number of items equals the number of requirement and acceptance-criterion identifiers in the selected corpus, and the output contains no aggregate score or percentage. | Test |
