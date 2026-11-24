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
every solution reference to the artifact representing the evidence content
that the source GSN model selected. A shared SHA-256 content-identity field
identifies retained evidence bytes and the case content covered by signed
approval. It is the sole digest emitted by these exports; no repository
revision identity or source-tree digest is produced.

## Inputs

- The suite and selected run-record location on each GSN solution.
- The exact UTF-8 bytes of the retained run-record JSON file.
- The repository-relative run-record path.

## Outputs

- SACM Artifact elements in one ArtifactPackage, linked from solution
  ArtifactReferences.
- One stable content identity per distinct run-record byte sequence.

## Behavior

- The content identity SHALL be `sha256:<lowercase-hex>` over the exact bytes
  of the selected run-record file. It SHALL NOT be a run timestamp, Git object
  id, commit id, or identifier inferred from a Git repository.
- Each distinct content identity SHALL have one Artifact element. Two
  solutions that refer to identical bytes SHALL share that Artifact; distinct
  byte content SHALL produce distinct Artifact ids even when the suite id is
  equal.
- An Artifact SHALL expose the suite id, run timestamp as descriptive version
  metadata when present, recorded verdict, content identity, and a
  repository-relative location. The timestamp SHALL NOT participate in the
  Artifact id.
- Each solution's ArtifactReference SHALL point to the Artifact whose content
  identity matches its selected run record.
- When a run record is missing, the exporter SHALL retain that solution as
  undischarged evidence and SHALL NOT replace it with a timestamp or invented
  Artifact.
- The exporter SHALL NOT copy run contents into XMI, emit an absolute host
  path, inspect Git, or emit any additional digest or revision identifier.
- For verdicts `stale`, `suspect`, or `undischarged`, the exporter SHALL mark
  the evidence relation as non-supporting.
- For those same verdicts, the exporter SHALL mark the Claim as needing
  support and SHALL NOT convert the recorded verdict into an acceptance
  decision.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-228-AC-1 | Two solutions with identical run-record bytes share one Artifact; a third solution with different bytes gets a distinct Artifact even if its suite id matches. | Test |
| FR-228-AC-2 | Each Artifact exposes the selected content identity, suite id, verdict, relative run-record path, and timestamp as metadata only; its XMI id depends on content identity and not timestamp. | Test |
| FR-228-AC-3 | For stale, suspect, and undischarged evidence, the AssertedEvidence has `isCounter=false` and the linked Claim has `needsSupport=true`. | Test |
| FR-228-AC-4 | A solution with no run remains visible as undischarged evidence and has no invented Artifact. | Test |
| FR-228-AC-5 | The XMI contains no absolute host path or Git revision id, and it contains no digest other than the declared content identities. | Test |
| FR-228-AC-6 | The XMI does not embed any run-record bytes. | Test |
| FR-228-AC-7 | Re-export with unchanged evidence content produces byte-identical XMI and stable content-derived Artifact ids. | Test |

## Dependencies

- **Upstream**: [FR-221](./FR-221-gsn-model-export.md)
- **Downstream**: [PLAT-1649](https://linear.app/agent-ix/issue/PLAT-1649)
