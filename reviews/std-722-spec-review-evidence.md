---
id: SR-2026112016223475846355070
title: Evidence review of SACM signed acceptance
type: SpecReview
analysis: evidence
scope: agent-ix/quoin; spec/functional/FR-225-signed-acceptance-of-case-content.md,
  spec/functional/FR-226-sacm-export.md
review_set: subset
---

## Summary

The live `quoin advise --json` result accepts the authored Test class for FR-226-AC-9. The criterion gives the fixture observable assertions for both input cases.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS**

The advisor recommended property-based testing in the Test class for AC-9 because the property extractor classified the criterion as universal. The required paired fixture directly observes the acceptance label and path, inspects references, and compares the declarations of the same Claims. The independent reading of the normative XML establishes type legality for this review. FR-226-AC-7 defines the exporter’s separate model-conformance check. There is no test binding for AC-9 yet because the implementation is tracked on PLAT-1647.

## Assurance profiles

Reviewed both active profiles. `spec/assurance/AP-201-finding-quality.md` governs finding-quality measurement and reporting. `spec/assurance/AP-202-jev-lens-evaluation.md` governs live evaluation of semantic lenses. Neither profile applies to SACM export mapping, so this is a direct specification review.

## Scope

```yaml
scope:
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
- id: FR-226-AC-4
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: Re-export of unchanged GSN input produces byte-identical UTF-8 XMI with stable element and
    relationship ordering.
- id: FR-226-AC-7
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: Local expectations validate the XMI shape; the explicit metamodel-conformance check compares
    the output against the normative model at `QUOIN_SACM_METAMODEL` and rejects an unset path.
- id: FR-226-AC-9
  path: spec/functional/FR-226-sacm-export.md
  role: examined
  excerpt: A GSN case with verified `signed_acceptance` and an open claim yields exactly one labeled acceptance
    Artifact via the root's `artifactPackage` and its `artifactElement` containment. No argument element
    references it. Exporting the case with `signed_acceptance` omitted yields no acceptance Artifact and
    identical Claim declarations.
```
