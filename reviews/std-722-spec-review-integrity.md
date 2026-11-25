---
id: SR-2026112016223475846355067
title: Integrity review of SACM signed acceptance
type: SpecReview
analysis: integrity
scope: agent-ix/quoin; spec/functional/FR-225-signed-acceptance-of-case-content.md,
  spec/functional/FR-226-sacm-export.md
review_set: subset
---

## Summary

FR-225 delegates placement to FR-226 while retaining verification and metadata ownership. FR-226 names the validated GSN field, concrete Artifact type, containment, and argument-graph boundary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS**

| Requirement | Upstream contract | Verification |
| --- | --- | --- |
| FR-225 | FR-047 authored assurance argument and FR-221 GSN model | FR-225-AC-1, AC-4, AC-5 |
| FR-226 | FR-221 validated GSN, FR-225 verified acceptance, FR-228 evidence package, NFR-046 external metamodel | FR-226-AC-2, AC-9 |

The reference from FR-225 is a location cross-reference. FR-226 retains its existing requirement for FR-225, and the diff introduces no reciprocal dependency edge. FR-228 supplies solution-evidence rules and FR-225 supplies signed-acceptance identity and signer metadata. Containment and no-argument-reference obligations have separately observable outputs. Comparison with an export of the same model with the field omitted isolates the acceptance addition.

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
  excerpt: "- The exporter SHALL request or consume EA verification for an approval tag\n  whose signed\
    \ subject equals the computed case content identity. It SHALL\n  retain the signer, declared authority,\
    \ approval reference, and approval\n  instant supplied by EA."
- id: FR-225
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: context_only
  excerpt: "- The exporter SHALL emit an accepted record only when EA reports the tag\n  verified and\
    \ its subject exactly matches the computed identity. A missing,\n  malformed, unverifiable, expired,\
    \ or mismatched approval SHALL NOT produce\n  an accepted record.\n- If approval verification is unavailable\
    \ or fails, the case export SHALL\n  remain available without positive acceptance metadata and SHALL\
    \ report the\n  reason. It SHALL NOT infer acceptance from the presence of a signer name or\n  tag\
    \ file."
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
- id: FR-225-AC-1
  path: spec/functional/FR-225-signed-acceptance-of-case-content.md
  role: examined
  excerpt: A verified EA approval over the exact case content identity appears in both GSN and SACM output
    with signer, authority, approval reference, and approval instant preserved.
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
- id: FR-226-AC-2
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: An open or undeveloped goal maps to a claim needing support; it is never asserted, including
    when the source is a counter-claim.
- id: FR-226-AC-9
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: A GSN case with verified `signed_acceptance` and an open claim yields exactly one labeled acceptance
    Artifact via the root's `artifactPackage` and its `artifactElement` containment. No argument element
    references it. Exporting the case with `signed_acceptance` omitted yields no acceptance Artifact and
    identical Claim declarations.
```
