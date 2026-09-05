---
id: FR-088
title: "Measure the L3 semantic dimension, or record that it could not run"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-074"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-084"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-085"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-091"
    type: "depends_on"
---

# FR-088: Measure the L3 semantic dimension, or record that it could not run

## Description

The corpus measurement SHALL publish a per-document census of the `## Properties` authoring form over
the enumerated corpus, and SHALL record the field-level semantic conformance of a typed table as
`could-not-run` for every document while no released toolchain exposes the semantic extraction that
would decide it.

## Rationale

Nine of the ten measured modules declare object types and no Markdown mappings; their contract meets
the corpus at the L3 authoring form, so a corpus rate drawn only from the artifact modules says
nothing about them. The form census is Quoin's own, declared by FR-074. The field-level dimension is
not: resolving a `Type` token, a multiplicity or a constraint keyword belongs to Quire's extraction
and to `semantic-core`, and the released CLI is built against an engine revision that predates that
extraction. Reporting the field-level dimension as anything but `could-not-run` would be reporting a
check that never executed.

## Inputs

- The enumerated corpus documents of FR-084 — the same population, not a second walk.
- The resolved module set and toolchain record of FR-085.
- The declared tool-defect ledger of FR-091.

## Outputs

- One representation record per document: the classified form and the line of the first Properties
  block.
- A per-form census counting documents, stated per document.
- One field-level record per document carrying `could-not-run` and the citation that explains it,
  for as long as no released toolchain exposes the semantic extraction.

## Behavior

- The measurement SHALL classify each enumerated document's Properties representation as exactly one
  of `typed-table`, `free-column-table`, `bullet-list`, `sysml-fence` and `none`, using the classifier
  FR-074 declares.
- The measurement SHALL count the form census in documents.
- The measurement SHALL state the unit `documents` beside the form census.
- Where a document's classified form is `free-column-table` or `bullet-list`, the measurement SHALL
  record one advisory finding of class `unsupported-representation` against that document.
- The measurement SHALL NOT record an `unsupported-representation` finding as a conformance failure,
  because every measured module declares `legacy_forms: warning`.
- Where a document's classified form is `none`, the measurement SHALL record the document
  `not-applicable` for this check.
- The measurement SHALL record the field-level conformance of every document `could-not-run` while
  the toolchain record shows no engine surface for semantic extraction, citing the tool-defect ledger
  entry that states it.
- When a released toolchain exposes semantic extraction, the measurement SHALL obtain field-level
  conformance from that toolchain rather than from a classifier of its own.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-088-CON-1 | A `not-applicable` document SHALL be absent from this check's numerator and denominator. | Interface | Test |
| FR-088-CON-2 | The measurement SHALL NOT resolve a `Type` token, a multiplicity or a constraint keyword itself. | Interface | Inspection |
| FR-088-CON-3 | The form census and the corpus enumeration SHALL walk one population, never two. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-088-AC-1 | Each of the five representation forms is classified from a document exhibiting it, and each document carries exactly one form. | Test (TC-1525) |
| FR-088-AC-2 | The form census counts documents, states that unit, and its per-form counts sum to the enumerated document count. | Test (TC-1526) |
| FR-088-AC-3 | A free-column table and a bullet list each yield one `unsupported-representation` advisory finding, and neither is counted as a conformance failure. | Test (TC-1527) |
| FR-088-AC-4 | A document with no `## Properties` heading records `not-applicable` and is absent from both sides of this check's rate. | Test (TC-1528) |
| FR-088-AC-5 | Every document's field-level record is `could-not-run` carrying a tool-defect ledger citation while the toolchain record shows no semantic-extraction surface. | Test (TC-1529) |
| FR-088-AC-6 | The document set the form census classified is the same set FR-084 enumerated, compared element by element. | Test (TC-1530) |
| FR-088-AC-7 | No source file of the measurement resolves a `Type` token, a multiplicity form or a constraint keyword. | Inspection |

## Dependencies

- **Upstream**: [FR-074](./FR-074-legacy-authoring-forms.md) declares the classifier this reuses; [FR-084](./FR-084-pin-and-enumerate-the-governed-corpus.md) supplies the population; [FR-085](./FR-085-resolve-the-completed-module-set.md) supplies the toolchain record.
- **Downstream**: [FR-089](./FR-089-partition-every-failure.md), [FR-090](./FR-090-publish-rates-with-unit-population-and-method.md)
