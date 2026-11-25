---
id: FR-225
title: "Record a named person's signed acceptance of assurance-case content"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-047"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-221"
    type: "requires"
---

# FR-225: Record a named person's signed acceptance of assurance-case content

## Description

When a person accepts an assurance case, `quoin assurance` SHALL display the
person's Engineering Assurance (EA) approval only when the approval record is
verified for the exact case content identity. The export SHALL preserve who
accepted which content, the approval reference, and the recorded approval
instant. This records acceptance of the case representation; it does not
accept product risk or decide product, clinical, safety, or compliance
acceptability.

## Inputs

- The deterministic `quoin.gsn-case/v1` model before acceptance metadata is
  attached.
- An EA approval record whose signed subject names that model's content
  identity.
- The EA verifier's result and signer identity/authority fields.

## Outputs

- An optional top-level `signed_acceptance` record in GSN output and a
  distinct Artifact record in SACM output.
- The content identity and source EA approval reference to which it applies.

## Behavior

- The case content identity SHALL be `sha256:<lowercase-hex>` over the
  deterministic UTF-8 serialization of the GSN model with the
  `signed_acceptance` field omitted. This identity binds the case content and
  avoids a circular digest over the approval record itself.
- The exporter SHALL request or consume EA verification for an approval tag
  whose signed subject equals the computed case content identity. It SHALL
  retain the signer, declared authority, approval reference, and approval
  instant supplied by EA.
- The exporter SHALL emit an accepted record only when EA reports the tag
  verified and its subject exactly matches the computed identity. A missing,
  malformed, unverifiable, expired, or mismatched approval SHALL NOT produce
  an accepted record.
- If approval verification is unavailable or fails, the case export SHALL
  remain available without positive acceptance metadata and SHALL report the
  reason. It SHALL NOT infer acceptance from the presence of a signer name or
  tag file.
- The exporter SHALL invoke EA to verify signatures and signer authorization.
  Quoin SHALL NOT inspect Git or implement a second signer-verification
  policy.
- GSN and SACM output SHALL carry the same signed-acceptance content identity
  for the same input case. GSN output SHALL carry the acceptance as explicit
  metadata. SACM output SHALL carry it as a distinct Artifact record in the
  root case's ArtifactPackage as specified by [FR-226](./FR-226-sacm-export.md).
- The exporter SHALL label acceptance metadata as the human acceptance of the
  case content and SHALL NOT present it as evidence that establishes product
  safety or risk acceptability.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-225-AC-1 | A verified EA approval over the exact case content identity appears in both GSN and SACM output with signer, authority, approval reference, and approval instant preserved. | Test |
| FR-225-AC-2 | An approval tag for different content, an unverifiable tag, or an expired approval emits no accepted record and reports the corresponding reason. | Test |
| FR-225-AC-3 | When EA approval verification is unavailable, export contains no positive acceptance assertion and still preserves the underlying case. | Test |
| FR-225-AC-4 | The content identity is stable for byte-identical canonical GSN models, changes when case content changes, and is identical in GSN and SACM output for the same case. | Test |
| FR-225-AC-5 | Export performs no Git subprocess and does not assign product-risk, safety, or compliance acceptability from the human acceptance. | Test |

## Dependencies

- **Upstream**: [FR-221](./FR-221-gsn-model-export.md), [FR-211](./FR-211-discharge-resolves-evidence-and-approvals.md)
