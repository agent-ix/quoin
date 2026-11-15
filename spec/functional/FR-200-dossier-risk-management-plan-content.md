---
id: FR-200
title: "The dossier risk management plan renders the authored policy and states the missing plan record"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/spec-objects-safety/FR-101"
    type: "references"
---

# FR-200: The dossier risk management plan renders the authored policy and states the missing plan record

## Description

When `quoin dossier generate` selects `risk-management-plan` (document id
`DOS-RMP`), the risk management plan document SHALL render each authored
`acceptability_policy` and SHALL state the absence of a plan record.

## Plan-record decision

PLAT-1615 chooses option **(b)**: the existing Risk Model has an
`acceptability_policy` object but no object for plan scope, named
responsibilities or planned control verification. This project will render the
policy and keep `no_plan_record` as a permanent gap. It will not synthesize a
plan, add a new Risk Model object, or imply that the missing plan was approved.

## Inputs

- `acceptability_policy` records defined by spec-objects-safety
  [FR-101](ix://agent-ix/spec-objects-safety/FR-101), read through
  `quoin_quire::records::read` with that module selected. The existing reader
  used by `quoin_risk_advice::advise` in
  `rust/crates/quoin-risk-advice/src/lib.rs` is the source of record.
- The policy's severity levels, probability levels, regions with their
  `outside_acceptable` flags, and cells. No source contains a plan record.

## Outputs

- `dossier/DOS-RMP.md`: `# Risk management plan`; `## Scope`,
  `## Responsibilities`, `## Verification of risk controls`,
  `## Acceptability policy`, then `## Gaps`.
- The first three sections each state that no authored plan record exists.
  Each policy has a subsection in id order with levels, a
  `Region | Outside acceptable` table and a
  `Severity | Probability | Region` cell table.

## Behavior

- The document SHALL render each policy's levels, regions, flags and cells
  exactly as authored, in policy id order when more than one exists.
- The document SHALL NOT select one of several policies as in force.
- If no policy exists, the document SHALL produce `no_acceptability_policy`
  and SHALL show no example or default levels.
- The document SHALL always produce one document-level `no_plan_record` gap
  while the Risk Model declares no plan record. The three plan sections SHALL
  show the same missing-source condition, and the document SHALL be
  `incomplete`. Once registered, this makes unfiltered `generate` exit 1
  by FR-191 even if every other selected document is complete.
- When the policy reader reports an invalid record, including a region with
  no `outside_acceptable`, the dossier SHALL surface source exit 3 and
  write no selected document.
- The dossier SHALL NOT fill in a missing policy flag.
- The document SHALL render policy vocabulary only as policy data, without
  stating a new decision for any particular risk.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-200-AC-1 | A fixture with one policy renders every level, region, `outside_acceptable` flag and cell byte-equal to the record; it also has one `no_plan_record` gap and status `incomplete`. | Test |
| FR-200-AC-2 | With no policy, the document has `no_acceptability_policy` and `no_plan_record`, remains `incomplete`, and contains no example levels or cells. | Test |
| FR-200-AC-3 | Two policies appear in id order; neither is labelled the selected or in-force policy. | Test |
| FR-200-AC-4 | A policy whose region omits `outside_acceptable` causes dossier source exit 3 without a selected output file or default flag. | Test |
| FR-200-AC-5 | `Scope`, `Responsibilities` and `Verification of risk controls` each state that no authored plan record exists, without a placeholder name or planned activity. | Test |
| FR-200-AC-6 | Two runs over the same policy records produce byte-identical Markdown, and no line asserts that a particular residual risk is acceptable. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md) and
  spec-objects-safety [FR-101](ix://agent-ix/spec-objects-safety/FR-101).
- **Downstream**: PLAT-1656 implements this document.
