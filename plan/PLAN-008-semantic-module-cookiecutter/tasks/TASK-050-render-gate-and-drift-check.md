---
id: TASK-050
title: "Render gate, residue scan, conformance and drift check"
type: Task
status: done
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-048"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-049"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-083"
    type: references
  - target: "ix://agent-ix/quoin/NFR-018"
    type: references
  - target: "ix://agent-ix/quoin/NFR-019"
    type: references
  - target: "ix://agent-ix/quoin/NFR-020"
    type: references
  - target: "ix://agent-ix/quoin/TC-1412"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1418"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1444"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1445"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1446"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1447"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1448"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1455"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1462"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1464"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1465"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1466"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1470"
    type: verifies
---

# TASK-050: Render gate, residue scan, conformance and drift check

## Scope

Make the template verified by instantiation. Add the gate in this repository that
renders every variant into a temporary directory, checks it against the
conformance contract, scans it for residue, proves it renders and emits
deterministically, and fails when the maintained repositories carry a surface the
contract does not.

## Subtasks

- [x] `src/semantic/template.ts`: load and validate the conformance contract, resolve `{package}` segments, run the residue scan, and compare a rendered tree with the contract — a library surface this repository's suite covers directly.
- [x] `tests/semantic-module-template.test.ts`: render `artifact`, `object` and `mixed` unattended into temporary directories, and remove each afterwards including after a failure (TC-1444, TC-1445).
- [x] Fail naming the renderer, the floor and the install command when the renderer is absent; the same for the schema toolchain and the validator; report no skipped check in any case (TC-1464, TC-1448, TC-1465).
- [x] Residue scan over every rendered file at every depth, with a negative case per class injected into the template (TC-1446), and `.npmrc` absent everywhere (TC-1412).
- [x] Conformance check per variant, with a negative case removing a required surface (TC-1447).
- [x] Drift check against the maintained repositories at their pinned revisions, failing when one carries an unlisted, unexempted surface (TC-1418) or cannot be read at its revision (TC-1462).
- [x] Assert the template carries no copy of the emitter, the runtime or the grammar (TC-1455), and that a `⚠️` injected into a rendered Test Matrix fails the gate (TC-1470).
- [x] `make template-gate` in the Makefile, wired into this repository's `make test`, so the render gate is part of the green bar rather than a target somebody remembers.

## Deliverables

- A gate that would have caught every defect this template could ship.

## Notes

- A template that has never been instantiated is unverified. This task is the difference between the two.
