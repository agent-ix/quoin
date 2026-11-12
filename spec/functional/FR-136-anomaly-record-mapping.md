---
id: FR-136
title: "Anomaly document mapping to record keys and corpus-resolved assessment links"
type: FR
relationships:
  - target: "ix://agent-ix/spec-artifacts-process/FR-014"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-process/FR-015"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-121"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-123"
    type: "depends_on"
---

# FR-136: Anomaly document mapping to record keys and corpus-resolved assessment links

## Description

Quoin SHALL specify how an `Anomaly` document (declared by `agent-ix/spec-artifacts-process`
FR-014 and FR-015) maps to the record that quoin's consumers read, and how each safety-impact
assessment link is reconciled against the requirement and hazard ids of the corpus, so that the
`Anomaly` state, history and assessment are read by one rule, and a link that points at nothing is
reported by quoin's own commands as well as by `quire validate`. This requirement defines no
archetype, vocabulary or table header; those belong to the module.

## Rationale

The module owns what an anomaly document contains. Quoin owns the record key mapping (FR-121,
FR-123) and the corpus-level reading of references. Without a quoin mapping, the lifecycle and the
assessment links are visible only as validation text and cannot be traversed from a requirement or
a hazard back to the anomalies that affect them.

## Behavior

- Quoin SHALL map the `## History` table of an `Anomaly` to a `history` record key, one entry per
  row in authored order, with `from`, `to`, `by`, `date` and `note` taken verbatim from the cells,
  an empty `From` or `Note` yielding an absent field, and each entry carrying its 1-based line.
- Quoin SHALL map frontmatter `status` and `safety_impact` to record keys of the same name, carried
  verbatim, and SHALL NOT derive `status` from `history`.
- Quoin SHALL map each row of `## Safety Impact Assessment` to one `assessment` entry with `kind`,
  `target`, `impact` and `rationale`, and SHALL resolve a `Requirement` or `Hazard` row's `Target`
  to the `SemanticId` of the artifact it names in the corpus.
- If a `Target` resolves to no artifact, then Quoin SHALL keep the entry with `target` unresolved
  and report one dangling-reference diagnostic at the row, and SHALL NOT drop the entry or
  substitute another artifact.
- If a `Target` resolves to an artifact whose kind disagrees with the row's `Kind`, then Quoin SHALL
  report a kind-mismatch diagnostic at the row.
- Quoin SHALL expose each resolved assessment entry as a reverse reference, so that asking for the
  references to a requirement or hazard id lists the anomalies that name it, with the `Impact`.
- If an `Anomaly` has no `## History` or no `## Safety Impact Assessment` section, then Quoin SHALL
  report the corresponding record key `not_applicable` and SHALL NOT synthesise an empty one.
- Quoin SHALL read an `Anomaly` golden fixture from `agent-ix/spec-artifacts-process`, which owns
  it, and SHALL hold no copy.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-136-AC-1 | An `Anomaly` fixture with a three-row `## History` extracts `history` with three entries in order, with the first entry's `from` absent, and the record carries `status` and `safety_impact` equal to the frontmatter values. | Test |
| FR-136-AC-2 | A fixture whose frontmatter `status` differs from the last history `to` extracts both verbatim; `status` is not overwritten from `history`. | Test |
| FR-136-AC-3 | In a corpus holding `FR-001` and `HAZ-001`, an assessment with rows targeting both extracts two `assessment` entries whose `target` are the `SemanticId`s of those artifacts. | Test |
| FR-136-AC-4 | An assessment row targeting `FR-999` yields one dangling-reference diagnostic at that row and an entry with unresolved `target`; the other rows are unaffected. | Test |
| FR-136-AC-5 | A `Hazard` row targeting `FR-001` yields a kind-mismatch diagnostic at the row. | Test |
| FR-136-AC-6 | Asking for the references to `HAZ-001` lists the anomaly that names it with its `Impact`; asking for references to a hazard no anomaly names lists none. | Test |
| FR-136-AC-7 | An `identified` fixture with no assessment section extracts no `assessment` key and reports it `not_applicable`. | Test |
| FR-136-AC-8 | The fixture the tests read is resolved from the `agent-ix/spec-artifacts-process` checkout, and the quoin repository contains no copy of it. | Inspection |

## Dependencies

- **Upstream**: `agent-ix/spec-artifacts-process` [FR-014](ix://agent-ix/spec-artifacts-process/FR-014) and [FR-015](ix://agent-ix/spec-artifacts-process/FR-015) (the document shape), [FR-121](./FR-121-record-key-sections-mapping.md), [FR-123](./FR-123-model-table-sections-mapping.md)
- **Upstream, quire-rs**: the section and table extraction and reference resolution this mapping reads
- **Downstream**: the anomaly views and reports that read the record
