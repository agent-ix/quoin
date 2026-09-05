---
id: TASK-065
title: "The tool-defect ledger and its citations"
type: Task
status: todo
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-091"
    type: references
  - target: "ix://agent-ix/quoin/TC-1545"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1546"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1547"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1548"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1549"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1550"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1581"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1582"
    type: verifies
---

# TASK-065: The tool-defect ledger and its citations

## Scope

Declare the tool defects that distort this measurement as a section of the one
classification ledger, each citing a repository and issue number, each declaring
the scope it covers and the effect it has. A document inside a declared scope is
`could-not-run` for every check the entry blocks, whatever state it was assigned,
and a failure covered by an entry is classified `tool-defect` with the citation.

## Subtasks

- [ ] Author the ledger entries for agent-ix/quire-rs#402, agent-ix/quire-rs#403, agent-ix/spec-artifacts-process#81, agent-ix/quoin#347 and the absent `#388` CLI surface.
- [ ] Refuse an entry with no repository and issue number, naming the entry.
- [ ] Report affected counts in documents, and additionally in rows where the entry's effect is on rows.
- [ ] Publish the share of the population the declared exclusions cover, beside the aggregate rate.
- [ ] Raise an entry for every module capability the toolchain record shows no surface for.
- [ ] Never classify an undeclared failure as a tool defect.

## Deliverables

- A cited ledger, and a coverage statement saying how much of the corpus this measurement could not speak for.

## Notes

- quire-rs#403 makes an entire TypeScript file unreadable to the binder, so its rows report unbacked. Unbacked-because-unreadable is not untested, and the two must not share a bucket.
