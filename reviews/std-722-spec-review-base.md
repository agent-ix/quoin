---
id: SR-2026112016223475846355066
title: Base review of SACM signed acceptance
type: SpecReview
analysis: base
scope: agent-ix/quoin; spec/functional/FR-225-signed-acceptance-of-case-content.md,
  spec/functional/FR-226-sacm-export.md
review_set: subset
---

## Summary

Reviewed the signed-acceptance mapping in FR-225 and FR-226 for STD-722 at PR #1060 head `3115020f0259bef541681863a263679e7e28df07`. The containment path conforms to the OMG SACM 2.3 XML and the fixture criterion expresses the requested assertions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS**

The normative model declares `AssuranceCasePackage.artifactPackage : ArtifactPackage [0..*]` and `ArtifactPackage.artifactElement : ArtifactElement [0..*]` as composite. `Artifact` is an `ArtifactElement` through `ArtifactAsset`. `AssertedContext` inherits `source` and `target` from `AssertedRelationship`, both typed `ArgumentAsset`. Neither `Artifact` nor `AssuranceCasePackage` has that ancestor. The replacement path accepts the concrete classes and imposes one acceptance record when the verified field is present. The argument-reference ban also excludes an `ArtifactReference` because it inherits `ArgumentationElement`. [OMG SACM 2.3 normative XML](https://www.omg.org/spec/SACM/20220301/SACM.xml)

Both changed documents passed strict Quire validation. Requirement ids and AC ids are unique and sequence correctly. The added relative link resolves to FR-226. The existing SHA-256 requirement binds the EA approval to exactly the content it covers, which falls within the identity-digest exception.

The current matrix marks FR-226-AC-9 untagged. The paired fixture belongs to PLAT-1647. FR-225-AC-1 and FR-225-AC-4 cover the acceptance metadata and content identity separately. FR-226-AC-2 independently checks the open claim declaration, so the equality assertion in AC-9 cannot serve as the only oracle for the open state.

## Assurance profiles

Reviewed both active profiles. `spec/assurance/AP-201-finding-quality.md` governs finding-quality measurement and reporting. `spec/assurance/AP-202-jev-lens-evaluation.md` governs live evaluation of semantic lenses. Neither profile applies to SACM export mapping, so this is a direct specification review.

## Scope

```yaml
scope:
- id: FR-225
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: examined
  excerpt: "- GSN and SACM output SHALL carry the same signed-acceptance content identity\n  for the same\
    \ input case. GSN output SHALL carry the acceptance as explicit\n  metadata. SACM output SHALL carry\
    \ it as a distinct Artifact record in the\n  root case's ArtifactPackage as specified by [FR-226](./FR-226-sacm-export.md)."
- id: FR-226
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: "- When the GSN model contains `signed_acceptance`, the exporter SHALL place\n  exactly one\
    \ distinct Artifact labeled `Human acceptance of case content` in\n  the root AssuranceCasePackage's\
    \ `artifactPackage` via its `artifactElement`\n  containment. The Artifact SHALL preserve the acceptance\
    \ metadata specified\n  by [FR-225](./FR-225-signed-acceptance-of-case-content.md)."
- id: FR-226
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: "- The exporter SHALL NOT reference the acceptance Artifact from any argument\n  element."
- id: FR-226
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: "- The exporter SHALL leave every case Claim's assertion declaration unchanged\n  when it adds\
    \ signed-acceptance metadata."
- id: FR-225
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: context_only
  excerpt: "- The exporter SHALL emit an accepted record only when EA reports the tag\n  verified and\
    \ its subject exactly matches the computed identity. A missing,\n  malformed, unverifiable, expired,\
    \ or mismatched approval SHALL NOT produce\n  an accepted record.\n- If approval verification is unavailable\
    \ or fails, the case export SHALL\n  remain available without positive acceptance metadata and SHALL\
    \ report the\n  reason. It SHALL NOT infer acceptance from the presence of a signer name or\n  tag\
    \ file."
- id: FR-225
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: context_only
  excerpt: "- The exporter SHALL request or consume EA verification for an approval tag\n  whose signed\
    \ subject equals the computed case content identity. It SHALL\n  retain the signer, declared authority,\
    \ approval reference, and approval\n  instant supplied by EA."
- id: FR-225
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: context_only
  excerpt: "- The case content identity SHALL be `sha256:<lowercase-hex>` over the\n  deterministic UTF-8\
    \ serialization of the GSN model with the\n  `signed_acceptance` field omitted. This identity binds\
    \ the case content and\n  avoids a circular digest over the approval record itself."
- id: FR-225
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: examined
  excerpt: "- The exporter SHALL label acceptance metadata as the human acceptance of the\n  case content\
    \ and SHALL NOT present it as evidence that establishes product\n  safety or risk acceptability."
- id: FR-226
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: '| Artifact package | `sacm:ArtifactPackage` |'
- id: FR-226
  path: spec/functional/FR-226-sacm-export.md
  role: context_only
  excerpt: 'When the caller supplies `--sacm`, `quoin assurance` SHALL export the same

    derived case or authored argument represented by `quoin.gsn-case/v1` as an

    OMG SACM 2.3 AssuranceCasePackage serialized as UTF-8 XMI 2.5.1. The SACM

    export SHALL be built from the validated GSN model, preserving its case

    mapping, open states, source ids, and signed-acceptance metadata. It SHALL NOT

    build a second, independent mapping from the corpus.'
- id: FR-226
  path: spec/functional/FR-226-sacm-export.md
  role: context_only
  excerpt: "- A metamodel-conformance check MAY load the normative SACM 2.3 machine-readable\n  model\
    \ from the explicit `QUOIN_SACM_METAMODEL` path. That check SHALL fail\n  when the path is unset.\
    \ The metamodel source boundary is governed by\n  [NFR-046](../non-functional/NFR-046-no-copied-sacm-metamodel.md)."
- id: FR-225-AC-1
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: examined
  excerpt: A verified EA approval over the exact case content identity appears in both GSN and SACM output
    with signer, authority, approval reference, and approval instant preserved.
- id: FR-225-AC-2
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: examined
  excerpt: An approval tag for different content, an unverifiable tag, or an expired approval emits no
    accepted record and reports the corresponding reason.
- id: FR-225-AC-3
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: examined
  excerpt: When EA approval verification is unavailable, export contains no positive acceptance assertion
    and still preserves the underlying case.
- id: FR-225-AC-4
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: examined
  excerpt: The content identity is stable for byte-identical canonical GSN models, changes when case content
    changes, and is identical in GSN and SACM output for the same case.
- id: FR-225-AC-5
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: examined
  excerpt: Export performs no Git subprocess and does not assign product-risk, safety, or compliance acceptability
    from the human acceptance.
- id: FR-226-AC-1
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: A fixture containing each GSN node kind produces the stated SACM elements and relationships
    under an AssuranceCasePackage root, and all references resolve to an existing XMI id.
- id: FR-226-AC-2
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: An open or undeveloped goal maps to a claim needing support; it is never asserted, including
    when the source is a counter-claim.
- id: FR-226-AC-3
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: A direct goal-to-goal edge and a strategy-mediated edge produce their distinct AssertedInference
    structures with stable source ids.
- id: FR-226-AC-4
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: Re-export of unchanged GSN input produces byte-identical UTF-8 XMI with stable element and
    relationship ordering.
- id: FR-226-AC-5
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: Combining `--sacm` with any incompatible output flag returns a usage error and writes no XMI.
- id: FR-226-AC-6
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: A well-formedness failure in the GSN input writes no XMI and preserves the named WF rule and
    node ids in the error report.
- id: FR-226-AC-7
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: Local expectations validate the XMI shape; the explicit metamodel-conformance check compares
    the output against the normative model at `QUOIN_SACM_METAMODEL` and rejects an unset path.
- id: FR-226-AC-8
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: No external interop application is named as verified until PLAT-1649 has successful fixture-import
    evidence with application name, version, and licence.
- id: FR-226-AC-9
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: A GSN case with verified `signed_acceptance` and an open claim yields exactly one labeled acceptance
    Artifact via the root's `artifactPackage` and its `artifactElement` containment. No argument element
    references it. Exporting the case with `signed_acceptance` omitted yields no acceptance Artifact and
    identical Claim declarations.
```
