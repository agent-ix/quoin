---
id: FR-088
title: "Measure the L3 Properties representation of every measured document"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-074"
    type: "depends_on"
---

# FR-088: Measure the L3 Properties representation of every measured document

## Description

The corpus measurement SHALL classify the `## Properties` representation of every measured document
and, for a typed table, SHALL report each field row's conformance to the declared type, multiplicity
and constraint vocabularies as `pass`, `fail` or `could-not-run`.

## Rationale

The object-type modules of this wave declare no Markdown mappings; their contract meets the corpus at
the L3 authoring form. Measuring only the artifact modules would publish a corpus rate that says
nothing at all about nine of the ten modules, which is exactly the aggregate this campaign is
forbidden to publish.

## Inputs

- A measured document's bytes.
- The declared object types, enumerations and kernel scalars of the resolved module set.
- The declared multiplicity forms and the closed constraint keyword vocabulary.

## Outputs

- One representation record per measured document: the classified form, the line of the first
  Properties block, and the per-row findings of a typed table.
- A per-row finding naming the offending cell, its column and its document line.

## Behavior

- The measurement SHALL classify a document's Properties representation as exactly one of
  `typed-table`, `free-column-table`, `bullet-list`, `sysml-fence` or `none`.
- If a document carries both a Properties table and a `sysml` fence under the same heading, then the
  measurement SHALL report the document `fail` with the both-representations finding.
- Where the classified form is `typed-table`, the measurement SHALL resolve each row's `Type` cell
  against the declared object types, the declared enumerations and the declared kernel scalars.
- If a row's `Type` cell resolves to none of those, then the measurement SHALL report that row `fail`
  with the unresolved token and its line.
- Where the classified form is `typed-table`, the measurement SHALL report a row `fail` when its
  `Multiplicity` cell is outside the declared multiplicity forms.
- Where the classified form is `typed-table`, the measurement SHALL report a row `fail` when its
  `Constraints` cell uses a keyword outside the closed constraint vocabulary.
- Where the classified form is `free-column-table` or `bullet-list`, the measurement SHALL report the
  document `unsupported-representation` rather than `fail`, because the module accepts those forms at
  warning under a declared migration.
- Where a document carries no `## Properties` heading, the measurement SHALL report the document
  `not-applicable` for this check.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-088-CON-1 | A `not-applicable` document SHALL be absent from this check's numerator and denominator. | Interface | Test |
| FR-088-CON-2 | The constraint keyword vocabulary applied SHALL be the one the resolved module set declares, never a copy held by the measurement. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-088-AC-1 | Each of the five representation forms is classified from a document exhibiting it, and each document carries exactly one form. | Test (TC-1525) |
| FR-088-AC-2 | A document carrying both a Properties table and a `sysml` fence reports `fail` with the both-representations finding and both block lines. | Test (TC-1526) |
| FR-088-AC-3 | A typed-table row whose `Type` cell names no declared object type, enumeration or kernel scalar reports `fail` naming the token and its line. | Test (TC-1527) |
| FR-088-AC-4 | A typed-table row whose `Multiplicity` cell is `0..2..3` reports `fail`, and rows carrying `1`, `0..1`, `0..*`, `1..*` and `2..7` report `pass`. | Test (TC-1528) |
| FR-088-AC-5 | A typed-table row whose `Constraints` cell names a keyword outside the closed vocabulary reports `fail` naming that keyword. | Test (TC-1529) |
| FR-088-AC-6 | A free-column table and a bullet list each report `unsupported-representation`, and a document with no `## Properties` heading reports `not-applicable` and is absent from both the numerator and the denominator. | Test (TC-1530) |

## Dependencies

- **Upstream**: [FR-074](./FR-074-legacy-authoring-forms.md) declares the legacy-form classification this check reuses.
