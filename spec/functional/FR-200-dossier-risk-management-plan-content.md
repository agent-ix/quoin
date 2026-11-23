---
id: FR-200
title: "The dossier risk management plan renders the authored plan and policy and states a missing plan record"
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
  - target: "ix://agent-ix/spec-objects-safety/FR-108"
    type: "references"
---

# FR-200: The dossier risk management plan renders the authored plan and policy and states a missing plan record

## Description

When `quoin dossier generate` selects `risk-management-plan` (document id
`DOS-RMP`), the risk management plan document SHALL render each authored
`risk_management_plan` record and each authored `acceptability_policy`, and
SHALL state the absence of a plan record when none exists.

## Plan-record decision

PLAT-1615 chose to render the policy and keep `no_plan_record` as a gap while
the Risk Model had no object for plan scope, named responsibilities or planned
control verification. PLAT-1834 adds that object, spec-objects-safety
[FR-108](ix://agent-ix/spec-objects-safety/FR-108). `no_plan_record` is
therefore no longer permanent: it is reported exactly when no authored plan
record exists in scope. The dossier does not synthesize a plan, fill in a
missing plan row, or imply that a plan was approved. People accept risk; the
plan holds a reference to the acceptability criteria people wrote and the
dossier never assigns a score, a class or an acceptability.

## Inputs

- `risk_management_plan` records defined by spec-objects-safety
  [FR-108](ix://agent-ix/spec-objects-safety/FR-108), and `acceptability_policy`
  records defined by [FR-101](ix://agent-ix/spec-objects-safety/FR-101), read
  through `quoin_quire::records::read` with that module selected. The existing
  reader used by `quoin_risk_advice::advise` in
  `rust/crates/quoin-risk-advice/src/lib.rs` is the source of record.
- A plan's scope, responsibilities, control verification methods and production
  and post-production methods tables, and its `references` edges to policies.
- The policy's severity levels, probability levels, regions with their
  `outside_acceptable` flags, and cells.

## Outputs

- `dossier/DOS-RMP.md`: `# Risk management plan`; `## Scope`,
  `## Responsibilities`, `## Verification of risk controls`,
  `## Production and post-production methods`, `## Acceptability policy`, then
  `## Gaps`.
- With no plan record, each of the first four sections states that no authored
  plan record exists. With plan records, each section has one subsection per
  plan in id order, holding that plan's table.
- The `## Acceptability policy` section names, for each plan, the policies it
  references, then holds each policy as a subsection in id order with levels, a
  `Region | Outside acceptable` table and a `Severity | Probability | Region`
  cell table.

## Behavior

- The document SHALL render each plan's four tables exactly as authored, in plan id order when more than one exists, and SHALL NOT select one of several plans as in force.
- The document SHALL render each policy's levels, regions, flags and cells exactly as authored, in policy id order when more than one exists, and SHALL NOT select one of several policies as in force.
- The document SHALL show, for each plan, the policies in scope that its `references` edges name, and SHALL NOT copy the criteria into the plan or choose a policy for it.
- If no plan record exists, then the document SHALL produce one document-level `no_plan_record` gap and the four plan sections SHALL state the same missing-source condition.
- If a plan record exists, then the document SHALL NOT produce `no_plan_record`.
- If a plan references no policy in scope, then the document SHALL produce a `no_acceptability_policy` gap naming that plan.
- If no policy exists, the document SHALL produce a document-level `no_acceptability_policy` gap and SHALL show no example or default levels.
- The document SHALL be `incomplete` when it has any gap. Unfiltered `generate` then exits 1 by FR-191.
- When the reader reports an invalid record, including a policy region with no `outside_acceptable`, or a plan table that is missing, has no row or has a blank cell, the dossier SHALL surface source exit 3 and write no selected document.
- The dossier SHALL NOT fill in a missing policy flag or a missing plan cell.
- The document SHALL render plan and policy vocabulary only as authored data, without stating a decision for any particular risk.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-200-AC-1 | A fixture with one policy renders every level, region, `outside_acceptable` flag and cell byte-equal to the record; it also has one `no_plan_record` gap and status `incomplete`. | Test |
| FR-200-AC-2 | With no policy, the document has `no_acceptability_policy` and `no_plan_record`, remains `incomplete`, and contains no example levels or cells. | Test |
| FR-200-AC-3 | Two policies appear in id order; neither is labelled the selected or in-force policy. | Test |
| FR-200-AC-4 | A policy whose region omits `outside_acceptable` causes dossier source exit 3 without a selected output file or default flag. | Test |
| FR-200-AC-5 | With no plan record, `Scope`, `Responsibilities`, `Verification of risk controls` and `Production and post-production methods` each state that no authored plan record exists, without a placeholder name or planned activity. | Test |
| FR-200-AC-6 | Two runs over the same policy records produce byte-identical Markdown, and no line asserts that a particular residual risk is acceptable. | Test |
| FR-200-AC-7 | A fixture with one plan and the policy it references renders the plan's scope, responsibilities, control verification and production and post-production rows byte-equal to the record, has no `no_plan_record` gap and no gap at all, and is `complete`. | Test |
| FR-200-AC-8 | With two policies, a plan that references one names only that one, and both policies still render in id order. | Test |
| FR-200-AC-9 | A plan that references no policy in scope yields one `no_acceptability_policy` gap whose subject is the plan, and no `no_plan_record` gap. | Test |
| FR-200-AC-10 | A plan with a missing table, a table with no row, or a blank cell causes dossier source exit 3 without a rendered row. | Test |
| FR-200-AC-11 | Two plans appear in id order, neither labelled in force, byte-identically across runs, with no approval or acceptance wording. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md) and
  spec-objects-safety [FR-101](ix://agent-ix/spec-objects-safety/FR-101) and
  [FR-108](ix://agent-ix/spec-objects-safety/FR-108).
- **Downstream**: PLAT-1656 and PLAT-1834 implement this document.
