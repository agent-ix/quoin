---
id: StR-010
title: "Reviewers receive submission documents generated from the repository's records"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-191"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-192"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-193"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-194"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-195"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/NFR-040"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-196"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-197"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-198"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-199"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-200"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-201"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/NFR-041"
    type: "satisfied_by"
---

# StR-010: Reviewers receive submission documents generated from the repository's records

## Stakeholder Need

A team that must hand a reviewer a submission dossier needs Quoin to write
its selected requirements, architecture, trace, verification,
SOUP, anomaly, risk and release documents from records the toolchain already
holds, list every input that is missing, and never fill a gap with a value no
record states, so that the reviewer reads documents that match one commit and
every gap is visible.

## Rationale

Documents written by hand drift from the requirements, design relations, matrix
and discharge records they restate, and a generator that supplies a plausible
value for a missing record reads as a decision someone made. A generated
document that ends in an explicit list of gaps lets the reviewer see what the
repository does not yet state.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-010-VC-1 | Every selected M1 or M2 document is generated from its declared source records and matches the commit it is exported from. | Test |
| StR-010-VC-2 | Every missing input appears as a named gap in the document and in the machine-readable report. | Test |
| StR-010-VC-3 | No criticality, risk, compliance or acceptability value appears that no source record states. | Test |
| StR-010-VC-4 | A missing M2 source input appears as a named gap in its document and the machine-readable report, and no release or risk verdict is supplied by the generator. | Test |

## Stakeholders

Assurance reviewers and regulatory submission owners who receive the documents.
Affected parties are the authors of the requirement, design and clause records
the documents restate.

## Dependencies

FR-191 owns the command; FR-192 to FR-195 own M1 content; FR-196 to FR-201
own M2 content; NFR-040 and NFR-041 constrain determinism and records-only
rendering.
