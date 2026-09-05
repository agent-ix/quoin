---
id: TASK-064
title: "The Properties form census and the L3 could-not-run"
type: Task
status: todo
track: B
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-088"
    type: references
  - target: "ix://agent-ix/quoin/TC-1525"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1526"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1527"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1528"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1529"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1530"
    type: verifies
---

# TASK-064: The Properties form census and the L3 could-not-run

## Scope

Classify every enumerated document's `## Properties` form with the classifier
FR-074 already declares, over the FR-084 population and not a second walk. A
free-column table or a bullet list is one advisory `unsupported-representation`
finding, never a conformance failure, because every measured module declares
`legacy_forms: warning`. The field-level dimension — resolving a `Type` token, a
multiplicity, a constraint keyword — is recorded `could-not-run` for every
document, citing the tool-defect entry that explains why.

## Subtasks

- [ ] Reuse `classifyArtifact` from `src/semantic/sweep.ts`; do not write a second classifier.
- [ ] Drive it from the FR-084 document list, and assert element-by-element that the two populations are one.
- [ ] Count the census in documents and state that unit beside it.
- [ ] Raise one `unsupported-representation` advisory finding per legacy-form document.
- [ ] Record `not-applicable` for a document with no `## Properties` heading and keep it out of both sides of the rate.
- [ ] Record every document's field-level conformance `could-not-run` with its citation while the toolchain record shows no semantic-extraction surface.

## Deliverables

- A per-document form census, and an honest statement of the dimension this toolchain cannot measure.

## Notes

- The released `quire` CLI 0.31.0 pins engine `ca7362d4`, which predates the `quire-rs#388` merge `17b80e4`, and exposes no semantic surface at all. Reporting the field-level dimension as anything but `could-not-run` would report a check that never ran.
