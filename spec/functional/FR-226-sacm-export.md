---
id: FR-226
title: "Export an assurance case as SACM 2.3 XMI"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-221"
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

- The validated `quoin.gsn-case/v1` model produced by [FR-221](./FR-221-gsn-model-export.md).
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
- A strategy SHALL map to ArgumentReasoning. A `supported_by` relation through
  a strategy SHALL map to an AssertedInference connecting the source and
  target Claims and identifying that reasoning. A direct goal-to-goal support
  relation SHALL map to an AssertedInference without a reasoning element.
- A solution SHALL map to an ArtifactReference linked to its source Claim by
  AssertedEvidence; its Artifact target and content identity are specified by
  [FR-228](./FR-228-sacm-evidence-artifact-content-identity.md).
- A context SHALL map to an ArtifactReference linked by AssertedContext. An
  assumption SHALL map to a Claim with an assumed declaration and an
  AssertedContext link. A justification SHALL map to a Claim with an asserted
  declaration and an AssertedContext link to the Claim or ArgumentReasoning it
  justifies.
- A counter-claim SHALL map to a Claim marked as needing support and an
  AssertedInference with `isCounter=true` from that Claim to the challenged
  Claim. It SHALL NOT map as contextual evidence or support. The associated
  top claim remains open.
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
- A metamodel-conformance check MAY load the normative SACM 2.3 machine-readable
  model from the explicit `QUOIN_SACM_METAMODEL` path. That check SHALL fail
  when the path is unset. The metamodel source boundary is governed by
  [NFR-046](../non-functional/NFR-046-no-copied-sacm-metamodel.md).
- The workspace has no XML writer dependency. If an XML writer is required,
  the implementation SHALL use `quick-xml` (MIT) as the sole new runtime
  dependency outside the workspace. The exact crate version is set in the
  Cargo manifest and lockfile.
- No external application candidate was found and verified for importing the
  SACM 2.3 fixture at specification time. The upstream ACME repository
  documents a SACM 2.3 EMF model but no verified importer
  ([source](https://github.com/SystemsAssuranceGroup/SACM)); the MMINT-A
  material documents GSN use but no SACM 2.3 XMI import
  ([source](https://github.com/ScienceOfComputerProgramming/SCICO-D-22-00363)).
  Headless EMF loading alone is not import interoperability evidence
  ([source](https://eclipse.dev/emf/markdown/?file=FAQ.md)).
  [PLAT-1649](https://linear.app/agent-ix/issue/PLAT-1649) SHALL name a
  candidate and confirm its version and licence only after a successful
  import of the fixture.

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
| Location-bearing artifact data | `sacm:Resource` |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-226-AC-1 | A fixture containing each GSN node kind produces the stated SACM elements and relationships under an AssuranceCasePackage root, and all references resolve to an existing XMI id. | Test |
| FR-226-AC-2 | An open or undeveloped goal maps to a claim needing support; it is never asserted, including when the source is a counter-claim. | Test |
| FR-226-AC-3 | A direct goal-to-goal edge and a strategy-mediated edge produce their distinct AssertedInference structures with stable source ids. | Test |
| FR-226-AC-4 | Re-export of unchanged GSN input produces byte-identical UTF-8 XMI with stable element and relationship ordering. | Test |
| FR-226-AC-5 | Combining `--sacm` with any incompatible output flag returns a usage error and writes no XMI. | Test |
| FR-226-AC-6 | A well-formedness failure in the GSN input writes no XMI and preserves the named WF rule and node ids in the error report. | Test |
| FR-226-AC-7 | Local expectations validate the XMI shape; the explicit metamodel-conformance check compares the output against the normative model at `QUOIN_SACM_METAMODEL` and rejects an unset path. | Test |
| FR-226-AC-8 | No external interop application is named as verified until PLAT-1649 has successful fixture-import evidence with application name, version, and licence. | Inspection |
| FR-226-AC-9 | A GSN case with verified `signed_acceptance` and an open claim yields exactly one labeled acceptance Artifact via the root's `artifactPackage` and its `artifactElement` containment. No argument element references it. Exporting the case with `signed_acceptance` omitted yields no acceptance Artifact and identical Claim declarations. | Test |

## Dependencies

- **Upstream**: [FR-221](./FR-221-gsn-model-export.md), [FR-222](./FR-222-gsn-model-well-formedness.md), [FR-223](./FR-223-assurance-case-context-and-counter-claims.md), [FR-225](./FR-225-signed-acceptance-of-case-content.md)
- **Downstream**: [PLAT-1647](https://linear.app/agent-ix/issue/PLAT-1647), [PLAT-1649](https://linear.app/agent-ix/issue/PLAT-1649)

## Reference

OMG Structured Assurance Case Metamodel (SACM), Version 2.3, formal October
2023: https://www.omg.org/spec/SACM/2.3. Normative machine-readable model:
https://www.omg.org/spec/SACM/20220301/SACM.xml. OMG XMI 2.5.1:
https://www.omg.org/spec/XMI/2.5.1. The requirement names the normative
metamodel element and type identifiers.
