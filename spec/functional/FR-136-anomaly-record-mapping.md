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
FR-014 and FR-015) maps to the record that quoin's consumers read, SHALL own the check of its
status moves over the git history of the file, and SHALL specify how each safety-impact assessment
link appears as a reverse reference, so that the lifecycle is judged by one rule and attributed by
the commits that made it. This requirement defines no archetype, vocabulary or table header; those
belong to the module. The reference resolution and kind checks of the assessment links are quire-rs
FR-026 and FR-040, named in spec-artifacts-process FR-015; this requirement does not re-implement
them.

## Rationale

The module cannot hold a typed history of who moved an anomaly and when without duplicating, and
being able to falsify, what git records. It therefore leaves transition legality to the one tool
that can read the history. Quoin owns the record key mapping (FR-121, FR-123) and the corpus-level
reading of references.

## Behavior

- Quoin SHALL map frontmatter `status`, `resolution`, `closed_by` and `safety_impact` to record
  keys of the same name, carried verbatim.
- Quoin SHALL derive a `statusHistory` record key from the git history of the file: one entry per
  commit that changed the value of `status`, in commit order, with `status`, `commit`, `author`,
  `authoredAt`, and the signature verdict git reports for that commit (`signed`, `unsigned` or
  `unknown`). The first entry is the commit that added the file.
- Quoin SHALL NOT derive `status` from `statusHistory`, and SHALL NOT take any author or date from a
  field typed into the document.
- Quoin SHALL check each consecutive pair of `statusHistory` entries against the legal moves of
  spec-artifacts-process FR-014 and report an illegal-transition diagnostic naming both commits for
  each illegal pair, together with the `resolution` of the later commit where the move is to
  `closed`.
- The first entry SHALL have `status` `identified`; otherwise Quoin SHALL report an
  illegal-first-state diagnostic.
- When a move is `resolved` to `assessed`, Quoin SHALL require that the text of the
  `## Safety Impact Assessment` section differs between the reopen commit and the commit of the next
  later move out of `assessed`, and SHALL otherwise report a reassessment-missing diagnostic naming
  the reopen commit.
- Quoin SHALL report a `statusHistory` key `not_applicable` and no transition diagnostic for a file
  that is not under git, and SHALL NOT synthesise an entry.
- Quoin SHALL map each row of `## Safety Impact Assessment` to one `assessment` entry with `kind`,
  `target`, `impact` and `rationale`, with `target` carried as the `SemanticId` quire-rs resolved.
- Quoin SHALL expose each resolved assessment entry as a reverse reference, so that asking for the
  references to a requirement or hazard id lists the anomalies that name it, with the `Impact`.
- If an `Anomaly` has no `## Safety Impact Assessment` section, then Quoin SHALL report the
  `assessment` key `not_applicable` and SHALL NOT synthesise an empty one.
- Quoin SHALL read an `Anomaly` golden fixture from `agent-ix/spec-artifacts-process`, which owns
  it, and SHALL hold no copy.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-136-AC-1 | An `Anomaly` fixture records `status`, `resolution`, `closed_by` and `safety_impact` equal to the frontmatter values. | Test |
| FR-136-AC-2 | A fixture repository whose commits set `status` to `identified`, `assessed`, `resolved`, `closed` extracts four `statusHistory` entries in order, each with the commit's author and authored date and a signature verdict, and no author or date typed in the document is read. | Test |
| FR-136-AC-3 | A history `identified`, `resolved` yields an illegal-transition diagnostic naming both commits; `assessed`, `closed` with `resolution: fixed` yields one; `identified`, `closed` with `duplicate` yields none. | Test |
| FR-136-AC-4 | A history whose first entry is not `identified` yields an illegal-first-state diagnostic. | Test |
| FR-136-AC-5 | A history `resolved`, `assessed`, `resolved` in which the assessment section is unchanged yields a reassessment-missing diagnostic naming the reopen commit, and the same history with an edited assessment section yields none. | Test |
| FR-136-AC-6 | A fixture file outside git reports `statusHistory` `not_applicable` and no transition diagnostic. | Test |
| FR-136-AC-7 | An assessment with rows targeting a requirement and a hazard extracts two `assessment` entries whose `target` are the resolved `SemanticId`s, and asking for the references to the hazard lists the anomaly with its `Impact`; a hazard no anomaly names lists none. | Test |
| FR-136-AC-8 | An `identified` fixture with no assessment section reports `assessment` `not_applicable`. | Test |
| FR-136-AC-9 | The fixture the tests read is resolved from the `agent-ix/spec-artifacts-process` checkout, and the quoin repository contains no copy of it. | Inspection |

## Dependencies

- **Upstream**: `agent-ix/spec-artifacts-process` [FR-014](ix://agent-ix/spec-artifacts-process/FR-014) and [FR-015](ix://agent-ix/spec-artifacts-process/FR-015) (the document shape), [FR-121](./FR-121-record-key-sections-mapping.md), [FR-123](./FR-123-model-table-sections-mapping.md)
- **Upstream, quire-rs**: [FR-026](ix://agent-ix/quire-rs/FR-026) (reference resolution) and [FR-040](ix://agent-ix/quire-rs/FR-040) (edge target typing) for the assessment links; the section and table extraction this mapping reads
- **Downstream**: the anomaly views and reports that read the record
