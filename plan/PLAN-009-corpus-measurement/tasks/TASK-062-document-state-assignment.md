---
id: TASK-062
title: "One measurement state per corpus document"
type: Task
status: todo
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-086"
    type: references
  - target: "ix://agent-ix/quoin/TC-1512"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1513"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1514"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1515"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1516"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1517"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1573"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1574"
    type: verifies
---

# TASK-062: One measurement state per corpus document

## Scope

Assign every enumerated document exactly one of `measured`, `out-of-model`,
`unreadable` and `contested`. Type resolution is case-sensitive and keyed on the
pair of module name and type name. A contested type — one two resolved modules
both declare — raises a `contract-defect` finding so that it reaches the partition
instead of disappearing between the states.

## Subtasks

- [ ] Read each document's frontmatter and resolve its `type` case-sensitively against the module-qualified vocabulary.
- [ ] Assign `out-of-model` with `no-declared-type` or `type-not-declared-by-any-module`, keeping the two reasons apart.
- [ ] Assign `unreadable` on a read failure or an unterminated frontmatter fence, and keep enumerating siblings.
- [ ] Assign `contested` on a type two modules declare, and raise one `contract-defect` finding naming them.
- [ ] Assert the four states are exhaustive, mutually exclusive, and sum to the FR-084 document count.

## Deliverables

- A state record per document, with no document unaccounted for and none counted twice.

## Notes

- `out-of-model` is not a failure of anything. The corpus holds far more untyped Markdown than typed, and a measurement that read that as failure would be measuring the wrong thing.
