---
id: TASK-048
title: "Rendered skeletons, fixtures and verification suite"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-047"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-079"
    type: references
  - target: "ix://agent-ix/quoin/FR-080"
    type: references
  - target: "ix://agent-ix/quoin/TC-1411"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1420"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1421"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1422"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1423"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1424"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1425"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1426"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1427"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1428"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1429"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1430"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1431"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1450"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1460"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1469"
    type: verifies
---

# TASK-048: Rendered skeletons, fixtures and verification suite

## Scope

Render, for every exported type, the typed-table skeleton, its `sysml`-fence
alternate, and its `ocl` invariants; one negative fixture per declared failure
mode with a distinct `expect` identifier; a legacy-form fixture; and the suite
that treats the engine as a hard dependency.

## Subtasks

- [x] Skeletons with the exact four-column Properties header, substantive rows, and a body comment stating the manifest's extraction contract (TC-1420, TC-1411).
- [x] `sysml`-fence alternates declaring the same field names, types and multiplicities (TC-1421).
- [x] `## Invariants` with one `ocl` fence per `### <clauseId>` heading (TC-1423).
- [x] Negative fixtures, each with a distinct `expect` and a `because` (TC-1424), including the both-forms-in-one-document case (TC-1422).
- [x] A legacy-form fixture asserted to yield exactly one warning and no error (TC-1425).
- [x] For the artifact and mixed variants, mapping declarations covering every property of every exported model and one golden record per type, serialized with sorted keys and two-space indentation (TC-1426).
- [x] `tests/conftest.py`: import the engine and FAIL naming `make dev-quire` and `agent-ix/quire-rs#392` when it is absent, when it lacks `extract_semantic`, or when it is older than the declared floor (TC-1428, TC-1429, TC-1460); fail naming the install command when the grammar package is absent (TC-1450). No skip anywhere (TC-1427).
- [x] `make dev-quire`, and no engine dependency in the package metadata; warnings as errors (TC-1430, TC-1469).
- [x] `make gate`: validate, lint, schema drift check, suite — failing when any leg fails (TC-1431).

## Deliverables

- A rendered repository whose suite proves its own contract, and whose green means the checks ran.

## Notes

- A skipped row is not coverage. The skip is the failure mode this task exists to prevent, and it is the one an ordinary `pytest.importorskip` would introduce.
