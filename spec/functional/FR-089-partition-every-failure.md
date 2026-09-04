---
id: FR-089
title: "Partition every failure with an owner and a disposition"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-087"
    type: "depends_on"
---

# FR-089: Partition every failure with an owner and a disposition

## Description

The corpus measurement SHALL assign every reported failure exactly one class from `contract-defect`,
`legitimate-undeclared-value`, `malformed-document`, `missing-structure`,
`unsupported-representation`, `stale-module`, `tool-defect` and `unknown`, together with a named owner
and a disposition.

## Rationale

A list of failures nobody owns is a list nobody acts on. The campaign's exit condition is that every
failure has an owner and a disposition, and its safety rule is that a high count never justifies
weakening the rule that produced it — which is only checkable if the class that would justify a
contract change is recorded separately from the seven that would not.

## Inputs

- The evaluation records of FR-087 and the representation records of FR-088.
- A declared classification ledger: for each classified failure or failure group, its class, owner and
  disposition.

## Outputs

- One partition entry per failure: the failing document, the check, the class, the owner, the
  disposition, and the evidence the classification rests on.
- A partition summary counting failures by class and by owner.

## Behavior

- The measurement SHALL classify a failure `contract-defect` only when the declared ledger records
  that the module's declaration, and not the document, is wrong.
- The measurement SHALL classify a failure `legitimate-undeclared-value` only when the ledger records
  a corpus value that is correct and that the module does not declare.
- The measurement SHALL classify a failure `tool-defect` only when the ledger cites the defect by
  repository and issue number.
- The measurement SHALL assign the class `unknown` to every failure the ledger does not classify.
- The measurement SHALL NOT reclassify an `unknown` failure to any other class without a ledger entry.
- Each partition entry SHALL name an owner as a GitHub repository or a person, never as a role with
  no addressee.
- Each partition entry SHALL carry a disposition from `contract-fix-this-campaign`,
  `deferred-corpus-fix`, `accepted`, or `undispositioned`.
- If a partition entry's disposition is `deferred-corpus-fix`, then the entry SHALL name the later
  campaign that will carry it.
- The measurement SHALL report the count of failures whose class is `unknown` and the count whose
  disposition is `undispositioned` as headline figures of the partition summary.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-089-CON-1 | The eight classes SHALL be exhaustive and mutually exclusive over the reported failures. | Interface | Test |
| FR-089-CON-2 | A failure SHALL NOT be removed from the partition by any classification. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-089-AC-1 | Every reported failure appears in exactly one partition entry, and the partition entry count equals the reported failure count. | Test (TC-1531) |
| FR-089-AC-2 | A failure with no ledger entry is classified `unknown` with disposition `undispositioned`, and both counts appear in the partition summary. | Test (TC-1532) |
| FR-089-AC-3 | A ledger entry claiming `tool-defect` without a repository and issue number is refused, and its failures stay `unknown`. | Test (TC-1533) |
| FR-089-AC-4 | A `deferred-corpus-fix` disposition with no named later campaign is refused. | Test (TC-1534) |
| FR-089-AC-5 | Every partition entry names an owner, and an entry whose owner is absent or is a bare role is refused. | Test (TC-1535) |
| FR-089-AC-6 | Classifying a failure `contract-defect` requires a ledger entry recording that the declaration is wrong; without one the failure stays `unknown`. | Test (TC-1536) |
| FR-089-AC-7 | The partition summary's per-class counts sum to the reported failure count. | Test (TC-1537) |

## Dependencies

- **Upstream**: [FR-087](./FR-087-evaluate-declared-markdown-mappings.md), [FR-088](./FR-088-measure-the-l3-properties-representation.md)
