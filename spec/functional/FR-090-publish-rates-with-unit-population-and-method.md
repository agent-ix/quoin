---
id: FR-090
title: "Publish every rate with its unit, population and method"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-084"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-086"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-087"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-088"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-089"
    type: "depends_on"
---

# FR-090: Publish every rate with its unit, population and method

## Description

The corpus measurement SHALL publish every rate accompanied by the unit it counts, the population it
was computed over, and the method that produced it, and SHALL publish the same rate partitioned by
module, by declared type and by repository beside every aggregate.

## Rationale

This programme has withdrawn published headline figures twice, both times because a number appeared
without the population it counted. The campaign's own acceptance criterion — that no aggregate hides a
weak module or type partition — cannot be met by an aggregate that is printed alone.

## Inputs

- The corpus record of FR-084 and the module and toolchain records of FR-085.
- The state records of FR-086, the evaluation records of FR-087, the representation records of
  FR-088, and the partition of FR-089.
- A declared divergence margin, in percentage points.

## Outputs

- A rate record per published figure: numerator, denominator, unit, population identifier, method
  identifier, and the check the rate describes.
- A machine-readable index binding every figure the prose report prints to the result artifact and
  field it was taken from.
- The same rate partitioned by module, by declared type and by repository.
- A divergence list naming every partition whose rate falls below its aggregate by more than a
  declared margin.

## Behavior

- The measurement SHALL state, for every published rate, the unit its numerator and denominator
  count.
- The measurement SHALL state, for every published rate, the population identifier that names the
  corpus identifier, each repository's resolved commit, each module's resolved commit, the engine
  source revision and the state filter the denominator was drawn from.
- The measurement SHALL key every by-type breakdown on the pair of module name and type name.
- The measurement SHALL exclude `could-not-run` and `not-applicable` outcomes from both the numerator
  and the denominator of a pass rate.
- The measurement SHALL publish the count of `could-not-run` outcomes beside every rate that excluded
  them.
- Where two published rates count different populations, the measurement SHALL publish both with
  their populations and SHALL NOT combine them into one figure.
- The measurement SHALL name, in the divergence list, every module, type and repository partition
  whose rate falls more than the declared divergence margin below the aggregate rate it belongs to.
- The measurement SHALL publish a partition's rate even when that partition's denominator is one.
- Where a partition's denominator is zero, the measurement SHALL publish that partition with a
  denominator of zero and no rate value, rather than a rate of zero or an omitted row.
- The measurement SHALL publish the count of repositories recorded `clean: false` or `stable: false`
  beside every corpus-level rate.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-090-CON-1 | A rate SHALL NOT be published without a unit, a population identifier and a method identifier. | Interface | Test |
| FR-090-CON-2 | A partition SHALL NOT be omitted from a published breakdown because its denominator is small. | Interface | Test |
| FR-090-CON-3 | The divergence margin SHALL be a declared input, never a threshold compiled into the measurement. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-090-AC-1 | Every rate in the published report carries a unit, a population identifier and a method identifier. | Test (TC-1538) |
| FR-090-AC-2 | The population identifier of every rate names the corpus revision and the module revisions it was computed against. | Test (TC-1539) |
| FR-090-AC-3 | A corpus in which one document reports `could-not-run` yields a rate whose denominator excludes that document and a `could-not-run` count of one beside the rate. | Test (TC-1540) |
| FR-090-AC-4 | The mapping rate and the representation rate are published separately with their own populations and are never summed. | Test (TC-1541) |
| FR-090-AC-5 | A module partition whose rate is more than the declared margin below the aggregate appears in the divergence list naming both rates. | Test (TC-1542) |
| FR-090-AC-6 | A repository contributing one measured document appears in the by-repository breakdown with a denominator of one. | Test (TC-1543) |
| FR-090-AC-7 | Every declared type of every resolved module appears in the by-type breakdown keyed on module and type, including a type with no instance in the corpus, which is published with a denominator of zero and no rate value rather than omitted. | Test (TC-1544) |
| FR-090-AC-8 | Every figure the prose report prints appears in the machine-readable index naming the artifact and field it came from. | Test (TC-1578) |
| FR-090-AC-9 | Changing only the declared divergence margin changes the membership of the divergence list. | Test (TC-1579) |
| FR-090-AC-10 | The count of repositories recorded `clean: false` or `stable: false` is published beside every corpus-level rate. | Test (TC-1580) |

## Dependencies

- **Upstream**: [FR-089](./FR-089-partition-every-failure.md)
