---
id: TASK-067
title: "Rates, breakdowns, the report and the figure index"
type: Task
status: todo
track: D
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-090"
    type: references
  - target: "ix://agent-ix/quoin/NFR-023"
    type: references
  - target: "ix://agent-ix/quoin/TC-1538"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1539"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1540"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1541"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1542"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1543"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1544"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1563"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1564"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1565"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1578"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1579"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1580"
    type: verifies
---

# TASK-067: Rates, breakdowns, the report and the figure index

## Scope

Publish every rate with its unit, its population identifier and its method,
partitioned by module, by module-qualified type and by repository. `could-not-run`
and `not-applicable` leave both sides of a rate and are published beside it. A
zero-denominator partition is published with no rate value rather than omitted. A
partition more than the declared divergence margin below its aggregate is named in
the divergence list. Every figure the prose report prints is bound, in a
machine-readable index, to the artifact and field it came from.

## Subtasks

- [ ] Emit the rate records with numerator, denominator, unit, population identifier and method identifier.
- [ ] Build the population identifier from the corpus identifier, every repository commit, every module commit and the engine revision.
- [ ] Emit the by-module, by-type and by-repository breakdowns, keyed on module and type.
- [ ] Emit the divergence list from the declared margin, and prove the margin is read by changing it.
- [ ] Publish the structural rate and the form census separately, with their own units and populations, never summed.
- [ ] Publish the `clean: false` / `stable: false` repository count beside every corpus-level rate.
- [ ] Write the prose report, and the figure index binding each printed figure to its artifact and field.
- [ ] Add the check that recomputes every printed figure from the artifact it names.

## Deliverables

- A report whose every number can be re-derived by a reader who did not run it.

## Notes

- Two published figures in this programme have been withdrawn, both because a number appeared without the population it counted. The figure index exists so that class of defect is checkable by opening one file.
