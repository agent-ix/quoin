---
id: NFR-031
title: "Risk advice states what people wrote and never decides"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-145"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-146"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-147"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-148"
    type: "constrains"
  - target: "ix://agent-ix/spec-objects-safety/FR-104"
    type: "depends_on"
---

# NFR-031: Risk advice states what people wrote and never decides

## Statement

`quoin risk advise` SHALL print only values that a person entered, a policy cell a person wrote, or a link that exists in the records.
It SHALL NOT originate a verdict.

## Scope

- Applies to: `quoin risk advise`, in both formats.
- Out of scope: the content of the safety record schemas, which `agent-ix/spec-objects-safety` governs, and the chain report `quoin risk-summary`.

## Rationale

A tool that words a policy cell as "acceptable", or lists a "recommended decision", reads as a decision nobody
made, and people then defer to it. Every region must trace to the policy and every decision to a named person.
The wording is generic and the safety module's own.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Authored files changed or created by a run | 0 | 0 | Test |
| Output sentences using "accept" with the tool as the subject | 0 | 0 | Test |
| Score, rank, verdict, proposed-decision or priority keys emitted | 0 | 0 | Test |
| Differing output bytes across two runs on identical inputs | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-031-AC-1 | Over every fixture of FR-145 to FR-148, every file's bytes are identical before and after a run, and no file is created. | Test |
| NFR-031-AC-2 | In the Markdown over every fixture, the stem "accept" appears only in the attribution statement, in the policy-flag wording of FR-146, and in values copied from a record, and every region line matches "the policy marks this cell as <region>". | Test |
| NFR-031-AC-3 | The JSON of every section holds no key named or typed as a score, weight, total, rank, verdict, recommendation, proposal or priority. | Test |
| NFR-031-AC-4 | Over a fixture with every optional input absent, each printed value is `null`, an epistemic marker, a finding token or copied from a named record. | Test |
| NFR-031-AC-5 | Over a fixture with an all-acceptable policy and an `undecided` residual risk, no output states or implies the risk is accepted. | Test |
| NFR-031-AC-6 | Two runs over the same fixtures give byte-identical output in both formats. | Test |

## Verification

All criteria are tests over fixtures.

## Dependencies

- **Upstream**: [FR-145](../functional/FR-145-risk-advise-command.md), [FR-146](../functional/FR-146-risk-advise-policy-cells.md), [FR-147](../functional/FR-147-risk-advise-evidence-gaps.md), [FR-148](../functional/FR-148-risk-advise-decision-queue.md); [FR-104](ix://agent-ix/spec-objects-safety/FR-104).
