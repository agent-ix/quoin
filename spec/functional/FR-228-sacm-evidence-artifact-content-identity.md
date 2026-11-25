---
id: FR-228
title: "Identify SACM evidence artifacts by content"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-221"
    type: "requires"
---

# FR-228: Identify SACM evidence artifacts by content

## Description

The SACM ArtifactPackage SHALL identify each evidence artifact by the content
of its retained evidence run record, not by its timestamp. It SHALL connect
derived evidence references to the artifacts representing the retained run
content selected by the source GSN model. It SHALL preserve authored citations
and their evaluated state when the GSN model supplies no retained run selection.
A shared SHA-256 content-identity field
identifies retained evidence bytes and the case content covered by signed
approval. It is the sole digest emitted by these exports; no repository
revision identity or source-tree digest is produced.

## Inputs

- The `LeafEvidence` on a derived GSN solution or undeveloped obligation goal,
  including each binding's suite and available observation time.
- Authored citation references with the resolution or decision state preserved
  by [FR-221](./FR-221-gsn-model-export.md).
- The exact UTF-8 bytes of the retained run-record JSON file.
- The repository-relative run-record path.

## Outputs

- SACM Artifact elements in one ArtifactPackage, linked from derived evidence
  ArtifactReferences when selected retained runs exist.
- One stable content identity per distinct run-record byte sequence.

## Behavior

### Derived run evidence

- The content identity SHALL be `sha256:<lowercase-hex>` over the exact bytes
  of the selected run-record file. It SHALL NOT be a run timestamp, Git object
  id, commit id, or identifier inferred from a Git repository.
- Each distinct content identity SHALL have one Artifact element. Two
  solutions that refer to identical bytes SHALL share that Artifact; distinct
  byte content SHALL produce distinct Artifact ids even when the suite id is
  equal.
- An Artifact SHALL expose the suite id, run timestamp as descriptive version
  metadata when present, recorded verdict and content identity. The timestamp
  SHALL NOT participate in the Artifact id.
- Each derived evidence ArtifactReference SHALL point to the Artifacts for
  the exact retained runs available for its selected suites and times.
  When the evidence verdict is `bound`, all selected runs are available,
  and the evidence node is supported, the AssertedEvidence SHALL have
  `assertionDeclaration="asserted"`.
  When the GSN model records no time for a bound suite, or the corresponding
  run file is missing, the reference and its AssertedEvidence SHALL remain
  visible. Omit the missing Artifact and any link to it. Do not substitute a
  run at another time or from another suite.
- When an obligation goal is open, its AssertedEvidence SHALL have
  `assertionDeclaration="needsSupport"` and `isCounter="false"`, even
  when a run Artifact is available. Apply the same declaration to
  `stale`, `suspect` and `undischarged` evidence, to a missing verdict and to
  missing runs. Preserve any available Artifacts and the recorded verdict.
  The relationship description SHALL state why the evidence is non-supporting,
  including the missing run when applicable. The target Claim SHALL have
  `assertionDeclaration="needsSupport"`.
- The exporter SHALL NOT copy run contents into XMI, emit an absolute host
  path, inspect Git, or emit any additional digest or revision identifier.

### Authored citations

- When an authored citation has no retained run selection, its
  ArtifactReference SHALL carry the authored reference text in its `name`.
  Leave `referencedArtifactElement` absent. The exporter SHALL perform no
  retained-run lookup for this citation and emit no fabricated run Artifact
  or digest.
- A resolved top-claim citation or a citation from a supported current
  sufficiency decision SHALL keep its declared AssertedEvidence relation
  with `assertionDeclaration="asserted"`. An unresolved, stale or vacuous
  top-claim citation or a citation from an open sufficiency criterion SHALL
  have `assertionDeclaration="needsSupport"` and `isCounter="false"`,
  carrying its source reason. Preserve the authored Claim's status. The
  absence of a selected run does not reopen a supported authored Claim.
- The exporter SHALL NOT convert either kind of evidence into an acceptance
  decision.

### Retained run paths

- For every distinct repository-relative path to a retained run, the exporter
  SHALL write a `Resource` with that path in the `content` of its inherited
  `name` (a `LangString`). The run Artifact, its Resources and their
  `ArtifactGroup` SHALL be peers in the ArtifactPackage. The group's
  `artifactElement` references SHALL include that Artifact and its Resources.
- The [SACM 2.3 PDF](https://www.omg.org/spec/SACM/2.3/PDF) §12.10 describes
  a composite `Resource.location` of type `MultiLangString`. The published
  [UML XML](https://www.omg.org/spec/SACM/20220301/SACM.xml) omits that
  property. The path in `Resource.name` is Quoin's display representation.
  The exporter SHALL emit no `Resource.location` element or attribute.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-228-AC-1 | Two solutions with identical run-record bytes share one Artifact; a third solution with different bytes gets a distinct Artifact even if its suite id matches. | Test |
| FR-228-AC-2 | Each retained run Artifact exposes its content identity, suite id, verdict, and timestamp metadata. Its id depends on content identity. Its ArtifactGroup references the Artifact and the Resource for each distinct retained path. Each Resource exposes the repository-relative path in its `name` content and has no `location` property. | Test |
| FR-228-AC-3 | For stale, suspect, and undischarged derived evidence, the AssertedEvidence has `isCounter="false"` and `assertionDeclaration="needsSupport"`, names the reason, and links to a Claim declared `needsSupport`. If its selected record exists, the ArtifactReference still references that record's Artifact. | Test |
| FR-228-AC-4 | A derived bound suite with no observation time and one whose selected run file is missing both retain non-supporting AssertedEvidence. Neither references an invented Artifact or a substituted run. | Test |
| FR-228-AC-5 | The XMI contains no absolute host path or Git revision id, and it contains no digest other than the declared content identities. | Test |
| FR-228-AC-6 | The XMI does not embed any run-record bytes. | Test |
| FR-228-AC-7 | Re-export with unchanged evidence content produces byte-identical XMI and stable content-derived Artifact ids. | Test |
| FR-228-AC-8 | `quoin assurance --sacm` on a derived repository with a healthy retained run and an open obligation with a bound suite but no run emits a cited Artifact and asserted evidence for the healthy run. The open obligation has its own Claim declared `needsSupport` and a non-supporting AssertedEvidence relation with no invented Artifact. | Test |
| FR-228-AC-9 | `quoin assurance --argument <ID> --sacm` preserves an otherwise supported Claim and its resolved authored citation without a selected run. Changing the authored evidence index to leave that citation unresolved preserves the citation text, marks its evidence relation and Claim `needsSupport`, and preserves the failure reason. Neither export emits a fabricated run Artifact or digest for that citation. | Test |

## Dependencies

- **Upstream**: [FR-221](./FR-221-gsn-model-export.md)
- **Downstream**: [PLAT-1649](https://linear.app/agent-ix/issue/PLAT-1649)
