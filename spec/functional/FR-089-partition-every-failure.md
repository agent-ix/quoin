---
id: FR-089
title: "Partition every finding with an owner and a disposition"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-085"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-086"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-087"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-088"
    type: "depends_on"
---

# FR-089: Partition every finding with an owner and a disposition

## Description

The corpus measurement SHALL assign every finding exactly one class from `contract-defect`,
`legitimate-undeclared-value`, `malformed-document`, `missing-structure`,
`unsupported-representation`, `stale-module`, `tool-defect` and `unknown`, together with a named owner
and a disposition.

## Rationale

A list of failures nobody owns is a list nobody acts on. The campaign's exit condition is that every
failure has an owner and a disposition, and its safety rule is that a high count never justifies
weakening the rule that produced it — which is only checkable if the class that would justify a
contract change is recorded separately from the seven that would not. Because a ledger is
hand-maintained data, every rule below is written so that an unstated case raises the `unknown` count
rather than lowering it.

## Inputs

- The finding stream: every `fail` outcome of FR-087, every advisory finding of FR-088, every module
  finding of FR-085 and every contested-type finding of FR-086.
- A declared classification ledger keyed on the finding identity below.

## Outputs

- One partition entry per finding: the finding identity, the class, the owner, the disposition and
  the evidence the classification rests on.
- A partition summary counting findings by class, by owner and by disposition.
- An `unmatched-ledger-entries` list naming every ledger entry that classified no finding in this run.

## Behavior

- The measurement SHALL identify a finding by the tuple of repository, repository-relative document
  path, check name and diagnostic code, and SHALL NOT include a document line number in that identity.
- The measurement SHALL classify a finding from the ledger entry whose key equals the finding
  identity, or from a ledger entry whose key is a declared prefix of it.
- The measurement SHALL classify a finding `malformed-document` when the ledger records that the
  document violates a form its own module declares and the declaration is agreed correct.
- The measurement SHALL classify a finding `missing-structure` when the ledger records that the
  document omits a structure its module declares required.
- The measurement SHALL classify a finding `stale-module` when the ledger records that the document
  conforms to an earlier released version of the same module.
- The measurement SHALL classify a finding `contract-defect` only when the ledger records that the
  module's declaration, and not the document, is wrong.
- The measurement SHALL classify a finding `legitimate-undeclared-value` only when the ledger records
  a corpus value that is correct and that the module does not declare.
- The measurement SHALL classify a finding `tool-defect` only when the ledger cites the defect by
  repository and issue number.
- The measurement SHALL assign the class `unknown` to every finding no ledger entry matches.
- The measurement SHALL NOT reclassify an `unknown` finding without a matching ledger entry.
- The measurement SHALL name an owner as a GitHub repository or a person, never as a role with no
  addressee.
- The measurement SHALL carry a disposition from `contract-fix-this-campaign`, `deferred-corpus-fix`,
  `accepted` and `undispositioned` on every partition entry.
- If a partition entry's disposition is `contract-fix-this-campaign`, then the entry SHALL name the
  module repository that owns the declaration to be amended.
- If a partition entry's disposition is `deferred-corpus-fix`, then the entry SHALL name the later
  campaign that will carry it.
- If a partition entry's disposition is `accepted`, then the entry SHALL name the human who accepted
  it and the promotion gate the acceptance is recorded against.
- The measurement SHALL report every ledger entry that matched no finding in the
  `unmatched-ledger-entries` list.
- The measurement SHALL report the count of findings whose class is `unknown`, the count whose
  disposition is `undispositioned`, and the count of unmatched ledger entries as headline figures of
  the partition summary.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-089-CON-1 | The eight classes SHALL be exhaustive and mutually exclusive over the reported findings. | Interface | Test |
| FR-089-CON-2 | A finding SHALL NOT be removed from the partition by any classification. | Interface | Test |
| FR-089-CON-3 | A `contract-fix-this-campaign` disposition SHALL NOT relax a declared constraint in order to lower a count; it amends a declaration the measurement showed to be wrong. | Safety | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-089-AC-1 | Every reported finding appears in exactly one partition entry, and the partition entry count equals the reported finding count. | Test (TC-1531) |
| FR-089-AC-2 | A finding no ledger entry matches is classified `unknown` with disposition `undispositioned`, and both counts appear in the partition summary. | Test (TC-1532) |
| FR-089-AC-3 | A ledger entry claiming `tool-defect` without a repository and issue number is refused, and its findings stay `unknown`. | Test (TC-1533) |
| FR-089-AC-4 | A `deferred-corpus-fix` disposition with no named later campaign is refused, a `contract-fix-this-campaign` with no named module repository is refused, and an `accepted` with no named human is refused. | Test (TC-1534) |
| FR-089-AC-5 | Every partition entry names an owner, and an entry whose owner is absent or is a bare role is refused. | Test (TC-1535) |
| FR-089-AC-6 | Classifying a finding `contract-defect` requires a ledger entry recording that the declaration is wrong; without one the finding stays `unknown`. | Test (TC-1536) |
| FR-089-AC-7 | The per-class counts sum to the reported finding count. | Test (TC-1537) |
| FR-089-AC-8 | Moving a finding's document line without changing its diagnostic keeps the ledger match, and changing its diagnostic code loses it and returns the finding to `unknown`. | Test (TC-1575) |
| FR-089-AC-9 | A ledger entry that matches no finding appears in `unmatched-ledger-entries` and in the headline count. | Test (TC-1576) |
| FR-089-AC-10 | A finding of each of `malformed-document`, `missing-structure` and `stale-module` is produced from a ledger entry recording that condition, and none of the three can be assigned without one. | Test (TC-1577) |

## Dependencies

- **Upstream**: [FR-085](./FR-085-resolve-the-completed-module-set.md), [FR-086](./FR-086-assign-one-measurement-state-per-document.md), [FR-087](./FR-087-measure-structural-conformance-through-the-engine.md), [FR-088](./FR-088-measure-the-l3-semantic-dimension.md)
- **Downstream**: [FR-090](./FR-090-publish-rates-with-unit-population-and-method.md)
