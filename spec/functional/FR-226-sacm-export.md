---
id: FR-226
title: "Export an assurance case as SACM 2.3 XMI"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-234"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-222"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-223"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-225"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-228"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-227"
    type: "requires"
---

# FR-226: Export an assurance case as SACM 2.3 XMI

## Description

When the caller supplies `--sacm`, `quoin assurance` SHALL export the same
derived case or authored argument represented by `quoin.gsn-case/v1` as an
OMG SACM 2.3 AssuranceCasePackage serialized as UTF-8 XMI 2.5.1. The SACM
export SHALL be built from the validated GSN model, preserving its case
mapping, open states, source ids, and signed-acceptance metadata. It SHALL NOT
build a second, independent mapping from the corpus.

## Inputs

- The validated `quoin.gsn-case/v1` model produced by [FR-234](./FR-234-gsn-model-export.md).
- The SACM ArtifactPackage evidence mapping in
  [FR-228](./FR-228-sacm-evidence-artifact-content-identity.md).

## Outputs

- One deterministic SACM 2.3 XMI document with one root AssuranceCasePackage.
- A report for unreachable and unmapped case content, as specified by
  [FR-227](./FR-227-sacm-unmapped-report.md).

## Behavior

- `--sacm` SHALL be mutually exclusive with `--json`, `--gsn`, and
  `--gsn-diagram`; an invalid combination is a usage error and writes no XMI.
- The exporter SHALL map a goal to a SACM Claim with an asserted declaration
  only when its source GSN node is not open.
- When a goal is undeveloped or open, the exporter SHALL mark its SACM Claim
  as needing support and SHALL NOT assert that Claim.
- A strategy SHALL map to ArgumentReasoning. A direct goal-to-goal
  `supported_by` edge SHALL map to an AssertedInference from the child Claim
  to the parent Claim. A two-edge goal-to-strategy-to-goal path SHALL map to
  the same Claim endpoints with `reasoning` set to that strategy's
  ArgumentReasoning.
- A direct goal-to-solution `supported_by` edge SHALL map to an
  AssertedEvidence from the solution's ArtifactReference to the goal's Claim.
  A two-edge goal-to-strategy-to-solution path SHALL map to the same endpoints
  with `reasoning` set to that strategy's ArgumentReasoning. The content and
  declaration of evidence relations follow
  [FR-228](./FR-228-sacm-evidence-artifact-content-identity.md).
- An open derived obligation goal with `LeafEvidence` SHALL retain its Claim.
  The exporter SHALL add an ArtifactReference for this evidence and an
  AssertedEvidence from that reference to the Claim, as specified by
  [FR-228](./FR-228-sacm-evidence-artifact-content-identity.md).
- The exporter SHALL report a strategy-to-strategy support edge as unmapped.
  When a support edge participates in none of the direct or two-edge mappings
  above, the exporter SHALL report it as unmapped. It SHALL NOT traverse an
  unmapped edge to construct an inference or evidence relation. Independently
  representable paths still map.
- A context SHALL map to an ArtifactReference linked by AssertedContext. An
  assumption SHALL map to a Claim with an assumed declaration and an
  AssertedContext link. A justification SHALL map to a Claim with an asserted
  declaration and an AssertedContext link to the Claim or ArgumentReasoning it
  justifies.
- A counter-claim SHALL map to a Claim with `assertionDeclaration="needsSupport"`.
  When it challenges a goal or assumption, the exporter SHALL emit an
  AssertedInference with `isCounter=true` from that Claim to the exact
  challenged Claim. The target retains its own declaration.
- When a counter-claim challenges a strategy, the exporter SHALL retain the
  counter-claim Claim and target ArgumentReasoning and report the challenge
  edge as unmapped. The exporter SHALL NOT retarget the challenge or emit a
  context or support relation for it. The top claim stays open for every open
  challenge. SACM 2.3 §11.14 gives AssertedInference Assertion endpoints.
  ArgumentReasoning is outside that hierarchy.
- When the GSN model contains `signed_acceptance`, the exporter SHALL place
  exactly one distinct Artifact labeled `Human acceptance of case content` in
  the root AssuranceCasePackage's `artifactPackage` via its `artifactElement`
  containment. The Artifact SHALL preserve the acceptance metadata specified
  by [FR-225](./FR-225-signed-acceptance-of-case-content.md).
- The exporter SHALL NOT reference the acceptance Artifact from any argument
  element.
- The exporter SHALL leave every case Claim's assertion declaration unchanged
  when it adds signed-acceptance metadata.
- The exporter SHALL order SACM elements deterministically from their source
  GSN node and edge order.
- The exporter SHALL derive XMI ids from source GSN ids with a stable role
  suffix for generated relationship elements.
- The exporter SHALL make every reference resolve to one id in the document.
- When the explicit metamodel check is run, it SHALL validate the emitted
  classes, properties, containment and reference types against the published
  SACM 2.3 UML XML at `QUOIN_SACM_METAMODEL`. It SHALL fail when the path is
  unset. The check uses the published model as supplied, including the absence
  of `Resource.location`. [FR-228](./FR-228-sacm-evidence-artifact-content-identity.md)
  specifies the path representation. The model source boundary is governed by
  [NFR-046](../non-functional/NFR-046-no-copied-sacm-metamodel.md).
- The workspace has no XML writer dependency. If an XML writer is required,
  the implementation SHALL use `quick-xml` (MIT) as the sole new runtime
  dependency outside the workspace. The exact crate version is set in the
  Cargo manifest and lockfile.
- The verified external importer is Assurance Forge `0.3.0-alpha.3`, an
  explicitly pre-release Linux x64 application under the MIT licence. The
  [release](https://github.com/lasrod/assurance-forge/releases/tag/0.3.0-alpha.3)
  reads and writes SACM 2.3 XMI; its
  [tagged licence](https://github.com/lasrod/assurance-forge/blob/0.3.0-alpha.3/LICENSE.md)
  is MIT. Its bundled headless `assurance-forge-mcp` interface opens the
  unchanged fixture with `--offline-project` and exposes `get_case_overview`
  and `get_element` reads. `make test-sacm-interop` uses this interface to
  reject load warnings, compare Claim, ArgumentReasoning, AssertedRelationship
  and Artifact counts with the XMI, and inspect claim declarations, evidence
  relationship endpoints, the ArtifactGroup element, and the exact Resource
  path. It fails if `QUOIN_SACM_TOOL` is unset or unusable, and verifies that
  a fixture with a dangling reference is rejected. Neither the application,
  its binaries, its libraries, nor its data is committed.
- The MCP read projection does not expose an ArtifactGroup's `artifactElement`
  member ids, so the interop report SHALL NOT claim that those ids were
  returned by Assurance Forge MCP. Separate release-bound evidence for this
  relationship is the Assurance Forge SACM library test
  [`SACM23_ART_001_FullArtifactFixtureRoundTrips`](https://github.com/lasrod/assurance-forge/blob/65577b306a5dadf1d250497d2ed6e16139870747/libs/sacm/tests/test_artifact.cpp#L35-L77):
  it strictly imports an XMI fixture,
  asserts two ArtifactGroup members, saves and reloads the model, and requires
  semantic equality. This verifies the library's ArtifactGroup reference
  round trip; it is not a claim of SACM-wide conformance. A generic XML or EMF
  load alone does not establish preservation by the external application.

## Namespaces and XMI types

The XMI document SHALL bind prefix `xmi` to
`http://www.omg.org/spec/XMI/20131001` and prefix `sacm` to
`http://www.omg.org/spec/SACM/20220301`. The root element SHALL use
`xmi:type="sacm:AssuranceCasePackage"`.

The exporter SHALL use these `xmi:type` identifiers from the normative SACM
2.3 model:

| SACM element role | `xmi:type` |
| --- | --- |
| Claim, including a goal or counter-claim | `sacm:Claim` |
| Strategy | `sacm:ArgumentReasoning` |
| Support or counter-claim relation | `sacm:AssertedInference` |
| Solution evidence relation | `sacm:AssertedEvidence` |
| Context relation | `sacm:AssertedContext` |
| Evidence reference | `sacm:ArtifactReference` |
| Artifact package | `sacm:ArtifactPackage` |
| Evidence or acceptance record | `sacm:Artifact` |
| Run path displayed as name metadata | `sacm:Resource` |
| Group associating a run Artifact with its path Resources | `sacm:ArtifactGroup` |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-226-AC-1 | A fixture containing each GSN node kind produces the stated SACM elements and relationships under an AssuranceCasePackage root, and all references resolve to an existing XMI id. | Test |
| FR-226-AC-2 | An open or undeveloped goal maps to a claim needing support; it is never asserted, including when the source is a counter-claim. | Test |
| FR-226-AC-3 | A direct goal-to-goal edge and a goal-to-strategy-to-goal path produce their respective AssertedInference structures with stable source ids. Both use the child and parent Claims as endpoints. Only the path sets `reasoning`, to the intervening ArgumentReasoning. | Test |
| FR-226-AC-4 | Re-export of unchanged GSN input produces byte-identical UTF-8 XMI with stable element and relationship ordering. | Test |
| FR-226-AC-5 | Combining `--sacm` with any incompatible output flag returns a usage error and writes no XMI. | Test |
| FR-226-AC-6 | A well-formedness failure in the GSN input writes no XMI and preserves the named WF rule and node ids in the error report. | Test |
| FR-226-AC-7 | The explicit check against the published UML XML at `QUOIN_SACM_METAMODEL` verifies the exported classes, containment and reference types, including `Resource.name` and `ArtifactGroup.artifactElement`. The check rejects an unset path and a fixture that invents a `Resource.location` property. | Test |
| FR-226-AC-8 | A verified importer claim identifies Assurance Forge `0.3.0-alpha.3` and its MIT licence; the unchanged Quoin fixture imports without warnings and preserves the specified claims, evidence links and declarations, and exact Resource names. The MCP projection's ArtifactGroup-member limitation is stated, and the importer's separate ArtifactGroup round-trip test is cited without claiming that MCP returned the member ids. | Test / Inspection |
| FR-226-AC-9 | A GSN case with verified `signed_acceptance` and an open claim yields exactly one labeled acceptance Artifact via the root's `artifactPackage` and its `artifactElement` containment. No argument element references it. Exporting the case with `signed_acceptance` omitted yields no acceptance Artifact and identical Claim declarations. | Test |
| FR-226-AC-10 | `quoin assurance --argument <ID> --sacm` with open challenges to a goal, assumption, and strategy preserves all three counter-claim Claims and the open top claim. Counter inferences target exactly the goal and assumption Claims. The strategy challenge produces an unmapped edge diagnostic with both source ids and no substitute relationship. | Test |

## Dependencies

- **Upstream**: [FR-234](./FR-234-gsn-model-export.md), [FR-222](./FR-222-gsn-model-well-formedness.md), [FR-223](./FR-223-assurance-case-context-and-counter-claims.md), [FR-225](./FR-225-signed-acceptance-of-case-content.md)
- **Downstream**: [PLAT-1647](https://linear.app/agent-ix/issue/PLAT-1647), [PLAT-1649](https://linear.app/agent-ix/issue/PLAT-1649)

## Reference

OMG Structured Assurance Case Metamodel (SACM), Version 2.3, formal October
2023: https://www.omg.org/spec/SACM/2.3. Normative machine-readable model:
https://www.omg.org/spec/SACM/20220301/SACM.xml. OMG XMI 2.5.1:
https://www.omg.org/spec/XMI/2.5.1. The requirement names the normative
metamodel element and type identifiers.
