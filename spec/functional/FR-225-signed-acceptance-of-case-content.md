---
id: FR-225
title: "Record a named person's signed acceptance of assurance-case content"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-047"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-221"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "depends_on"
---

# FR-225: Record a named person's signed acceptance of assurance-case content

## Description

When the caller selects an approval for a derived case or an authored argument,
`quoin assurance --gsn` and `quoin assurance --sacm` SHALL record the human
acceptance only after Engineering Assurance (EA) verifies the approval for the
exact case content and its signed meaning declares that acceptance. The
acceptance records a person's decision about the case representation.

## Inputs

- The validated `quoin.gsn-case/v1` model before acceptance metadata is attached.
- `--case-approval-tag <TAG>` selects one annotated Git tag by bare name or
  `refs/tags/<name>` in the repository selected by `--repo` (defaulting to
  the current directory). Omitting the option selects no approval.
- `--case-approval-verifier <ABSOLUTE-PATH>` names the EA executable for a
  selected tag.
- `--case-approval-allowed-signers <SOURCE>` names the caller's trust source
  for the selected tag. EA FR-041 accepts an absolute file path or a committed
  `<rev>:<path>` source in the selected repository.

## Outputs

- An optional top-level `signed_acceptance` record in GSN output and the
  corresponding Artifact in SACM output as specified by
  [FR-226](./FR-226-sacm-export.md). Each carries the EA result's `record_id`,
  `record_digest` (the case content identity), `tag` (the approval reference),
  `approver`, and `meaning`. The `signer_source` field holds EA's
  `signature.signer.source`. The signing-time fields are specified below.
- If a selected approval cannot be recorded, a diagnostic on stderr
  identifying the selection and the failure reason.

## Behavior

### Selection and subject

- When no tag is selected, the exporter SHALL omit `signed_acceptance`, skip
  case-approval verification, and issue no case-approval diagnostic.
- The three case-approval options apply to `--gsn` and `--sacm`. The exporter
  SHALL take case-approval executable and trust inputs only from the case
  options. It SHALL keep the existing evidence, formal-rule, and authored
  clause verification inputs independent of the case options.
- When a tag is selected, the exporter SHALL compute the case content
  identity as `sha256:` followed by the lowercase hexadecimal SHA-256 of the
  RFC 8785 canonical UTF-8 JSON of the validated GSN model with
  `signed_acceptance` omitted. Use the shared canonical encoder specified by
  [FR-098](./FR-098-semantic-and-identity-parity.md). The identity covers the
  entire unsigned model and preserves array order. GSN JSON presentation and
  SACM XMI serialization do not enter the computation.
- When a tag is selected and the case verifier and trust inputs are available,
  the exporter SHALL send one `engineering-assurance.approval-verify-request/v1`
  request on stdin to the selected executable's `approval-verify --request -`
  command. Set `repository` to the absolute directory for `--repo`, `tag` to
  the selected tag, `allowed_signers` to the case trust source, `record_id`
  to the fixed record kind `quoin.gsn-case/v1`, and `record_digest` to the
  computed case content identity. The digest distinguishes cases within this
  kind. EA verifies the tag's `Approved-Record-Id` and `Approved-Record-Digest`
  bindings to that pair.
- The exporter SHALL use the offline SSH path of EA FR-041 in accordance
  with [NFR-045](../non-functional/NFR-045-assurance-exports-run-offline.md).
  The request omits `gitsign` and `identity_registry`. A gitsign-signed tag is
  a failed selection. The case-approval check runs after model validation.

### Verified acceptance

- The exporter SHALL record acceptance only from an exit-zero, well-formed
  `engineering-assurance.approval-verify-result/v1` response with `status`
  `approved`. The response must name the selected tag (EA returns the bare
  name), echo the requested record id, digest and allowed-signers source,
  and report both bindings as `match`.
- The response must identify an SSH approver mapped by `allowed_signers` and
  carry exactly one `meaning` value, byte-for-byte `assurance-case-accepted`.
  The caller's case-specific trust source establishes who may accept the
  case content. EA supplies the verified principal and signed meaning.
  Quoin SHALL NOT infer a signer role from the tag, the authored argument,
  or another verification path.
- The exporter SHALL preserve EA's `signed_at` and `signed_at_source` as a
  signer-claimed time sourced from `tagger_date`, or `signed_at_missing`
  when EA cannot read the tagger date. It SHALL preserve EA's
  `trusted_at_missing` value `ssh_signature` for the offline SSH result.
  Missing time alone does not prevent acceptance. EA's trust verdict
  determines signing-key validity. Quoin SHALL apply no case-acceptance
  expiry based on `--as-of`, the local clock, or the tagger date.
- If the exporter cannot encode the subject or obtain qualifying approval,
  it SHALL preserve the underlying case export with acceptance omitted.
  This includes missing or malformed case inputs, unavailable EA, an EA
  refusal, and any response that fails an acceptance condition. The
  diagnostic SHALL identify the reason and include EA's status and reason
  or error code when EA supplies them.
- Quoin SHALL delegate signature checking and signer mapping to EA. The case
  export SHALL NOT inspect Git directly.
- The exporter SHALL label the metadata as human acceptance of case content.
  It SHALL NOT use acceptance as evidence, change a claim's status, or assign
  product-risk, clinical, safety, or compliance acceptability.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-225-AC-1 | For both a derived case and an authored argument, real EA verification of an SSH-signed annotated tag over the exact case and acceptance meaning yields matching GSN and SACM acceptance metadata. The export names EA's mapped approver, authority source and bare tag and distinguishes the claimed signing time from the missing trusted time. | Test |
| FR-225-AC-2 | A selected tag that is absent, malformed, bound to a different record id or digest, or rejected by EA (including a key EA judges outside its validity window) leaves acceptance absent and reports the reason. The same holds for an EA `approved` result that carries a different or repeated signed meaning. | Test |
| FR-225-AC-3 | With no tag selection, export succeeds without acceptance, a case-verifier call, or a case-approval diagnostic. With a selected tag and missing case trust, an unavailable case verifier, or a malformed result, export still succeeds with the same underlying case and reports the failure on stderr. Legacy verification options alone supply no case trust, and case options alone supply no clause trust. | Test |
| FR-225-AC-4 | The identity matches an independent SHA-256 of the RFC 8785 bytes of the unsigned model for both formats. JSON presentation changes preserve the identity, and changing a case statement changes it and invalidates the previous approval. | Test |
| FR-225-AC-5 | Case approval runs offline through EA without Quoin inspecting Git. Selecting a gitsign tag yields the case without acceptance and starts no gitsign verifier. Valid acceptance leaves open claims, argument relationships, and evidence unchanged. An otherwise approved result with `signed_at_missing` still exports acceptance and reports the missing claimed and trusted times without substituting a time. | Test |

## Dependencies

- **Upstream**: [FR-221](./FR-221-gsn-model-export.md),
  [EA FR-041](ix://agent-ix/engineering-assurance/FR-041),
  [NFR-045](../non-functional/NFR-045-assurance-exports-run-offline.md)
