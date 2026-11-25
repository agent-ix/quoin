---
id: SR-2026112016223475846355069
title: Failure-domain review of SACM signed acceptance
type: SpecReview
analysis: failure-domain
scope: agent-ix/quoin; spec/functional/FR-225-signed-acceptance-of-case-content.md,
  spec/functional/FR-226-sacm-export.md
review_set: subset
---

## Summary

Reviewed negative acceptance flows, identity preservation, and the new containment path. An omitted or unverified acceptance has no positive Artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS**

FR-225 still validates exact case content identity and defines behavior when EA cannot verify approval. FR-226 consumes the resulting validated GSN model, so placement adds no verifier or external side effect. Exactly one typed Artifact is emitted when the verified field is present. The two composite associations retain it at the case level, and the prohibition on references from argument elements prevents a new argument path. The acceptance metadata does not affect Claim declarations.

## Assurance profiles

Reviewed both active profiles. `spec/assurance/AP-201-finding-quality.md` governs finding-quality measurement and reporting. `spec/assurance/AP-202-jev-lens-evaluation.md` governs live evaluation of semantic lenses. Neither profile applies to SACM export mapping, so this is a direct specification review.

## Scope

```yaml
scope:
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
  excerpt: "- The case content identity SHALL be `sha256:<lowercase-hex>` over the\n  deterministic UTF-8\
    \ serialization of the GSN model with the\n  `signed_acceptance` field omitted. This identity binds\
    \ the case content and\n  avoids a circular digest over the approval record itself."
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
- id: FR-226
  path: spec/functional/FR-226-sacm-export.md
  role: context_only
  excerpt: 'When the caller supplies `--sacm`, `quoin assurance` SHALL export the same

    derived case or authored argument represented by `quoin.gsn-case/v1` as an

    OMG SACM 2.3 AssuranceCasePackage serialized as UTF-8 XMI 2.5.1. The SACM

    export SHALL be built from the validated GSN model, preserving its case

    mapping, open states, source ids, and signed-acceptance metadata. It SHALL NOT

    build a second, independent mapping from the corpus.'
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
- id: FR-226-AC-9
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: A GSN case with verified `signed_acceptance` and an open claim yields exactly one labeled acceptance
    Artifact via the root's `artifactPackage` and its `artifactElement` containment. No argument element
    references it. Exporting the case with `signed_acceptance` omitted yields no acceptance Artifact and
    identical Claim declarations.
```
