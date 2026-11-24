---
id: FR-214
title: "Risk decision attribution: resolve the signer of the commit that last set each decision"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-136"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-163"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-103"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-106"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-107"
    type: "depends_on"
---

# FR-214: Risk decision attribution: the signature state of the commit that last set each decision, and its mismatch with the named decider

## Description

Tracked by Linear PLAT-1815. A residual-risk, benefit-risk or overall-evaluation record names its decider as typed text
(`Decided by`), which anyone can type. As [FR-136](./FR-136-anomaly-record-mapping.md) does for an anomaly's status and
[FR-163](./FR-163-anomaly-report-release-rationale.md) for its release rationale, the `decision_attribution` section of
`quoin risk advise` ([FR-145](./FR-145-risk-advise-command.md)) reports what git recorded about the commit that last
set each decision: its author and its signature state. It reads no author, date or commit typed into the document.

The signature state is the reduction FR-136 states (`signed`, `unsigned`, `unknown`); expiry is not judged here.
When both `--engineering-assurance <absolute-path>` and `--allowed-signers <absolute-path>` are supplied to
`quoin risk advise`, it delegates signer verification to engineering-assurance `signature-verify` and reports
only the mapped principal that verifier returns for the exact decision commit. A principal is not inferred from
the commit author, email, or signature letter. With this verification configured, the typed decider is compared
with that principal; without it, the existing comparison with the commit author remains. An unmapped or
untrusted signer has no principal and does not fall back to the author for this comparison. The report does not
say the decision is valid or invalid.

## Behavior

- The command SHALL list one entry for every `residual_risk`, `benefit_risk` and `overall_residual_risk_evaluation` in scope whose `decision` is `accepted` or `not_accepted`, ordered by kind (in that order) then record id.
- Each entry SHALL have exactly the keys `record`, `kind`, `decision`, `decided_by` (the `Decided by` cell as written, `null` when empty), `decision_change`, `signer_principal` and `findings`.
- `decision_change` SHALL be the object FR-163 defines for `rationaleChange` (`state`, `commit`, `author`, `authoredAt`, `signature`, `signatureCode`), derived from the most recent commit on the first-parent history of the default branch, following renames, that changed the text of the record's `Decision` cell. Its state is `present` only where the working-tree decision equals the decision at the branch tip, `pending` where it differs, and `not-applicable` where the file is not under git.
- `signature` and `signatureCode` SHALL be derived as FR-136 derives them.
- `quoin risk advise` SHALL accept `--engineering-assurance` and `--allowed-signers` together or neither; each supplied value SHALL be an absolute path.
- When both options are supplied and `decision_change` is `present`, the command SHALL call engineering-assurance `signature-verify` for that exact commit in the decision record's repository, using the supplied allowed-signers file.
- The command SHALL report `signer_principal` as the mapped principal returned for that exact commit, and `null` when no mapped principal is returned or signature verification is not configured.
- When signature verification is not configured, if `decision_change` is `present` and `decided_by` is non-empty and differs from the commit `author` after trimming and ASCII case folding, then the entry SHALL carry one `decided-by-differs-from-commit-author` finding naming `decided_by` then the author.
- When signature verification is configured and returns a mapped principal, if `decision_change` is `present` and `decided_by` is non-empty and differs from that principal after trimming and ASCII case folding, then the entry SHALL carry one `decided-by-differs-from-verified-signer` finding naming `decided_by` then the principal.
- When signature verification is configured and returns no mapped principal, the command SHALL NOT compare `decided_by` with the commit author.
- A verifier refusal for an invalid allowed-signers file SHALL exit 2 without a report; any other verifier failure or a result for a different commit SHALL exit 3 without a report.
- If `decision_change` is `present` and its `signature` is not `signed`, then the entry SHALL carry one `decision-commit-not-signed` finding naming the commit.
- For a `pending` decision change, the command SHALL NOT emit a finding.
- For a `not-applicable` decision change, the command SHALL NOT emit a finding.
- The command SHALL NOT read a hazard's `status` or any date typed into a record.
- The command SHALL NOT state a decision valid, invalid, authorised or accepted by its own words; the finding tokens are the only wording it adds.
- If git cannot be run for a file under a repository, then the command SHALL exit 2 naming the file.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-214-AC-1 | In a git fixture whose commit by author `Pat` set an `accepted` residual-risk decision with `Decided by` `Pat`, the entry's `decision_change` is `present` with that commit, author, authored date and signature letter, and carries no finding for an equal decider. | Test |
| FR-214-AC-2 | A decision whose typed decider is `Lee` while the commit author is `Pat` carries `decided-by-differs-from-commit-author` naming `Lee` and `Pat`; changing the case or surrounding spaces of the decider removes it. | Test |
| FR-214-AC-3 | An unsigned decision commit carries `decision-commit-not-signed`; signature letters map to `signed`, `unsigned` and `unknown` as in FR-136-AC-12. | Test |
| FR-214-AC-4 | A later commit that edits only the rationale leaves `decision_change` on the commit that set the decision; a later commit that changes the decision moves it. | Test |
| FR-214-AC-5 | An uncommitted working-tree decision gives `pending` and no finding; a file outside git gives `not-applicable` and no finding. | Test |
| FR-214-AC-6 | `benefit_risk` and `overall_residual_risk_evaluation` records are attributed as a residual risk is, and `undecided` records of every kind are absent from the section. | Test |
| FR-214-AC-7 | The keys of every entry equal `record`, `kind`, `decision`, `decided_by`, `decision_change`, `signer_principal`, `findings` exactly; `signer_principal` is `null` without a mapped principal. | Test |
| FR-214-AC-8 | With both verifier options absent, a decider that differs from the commit author gets the author-mismatch finding and `signer_principal: null`; with the options supplied, a `signed_valid` response for the exact commit with mapped principal `Pat` reports that principal and does not flag `Decided by: Pat`, while a different principal is named by `decided-by-differs-from-verified-signer`. | Test |
| FR-214-AC-9 | A signature-verify result for another commit, an invalid result envelope, or an unavailable verifier exits 3 without a report; an invalid allowed-signers file exits 2 without a report; an unmapped signer reports `signer_principal: null` and does not compare the decider with the commit author. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md), [FR-136](./FR-136-anomaly-record-mapping.md), [FR-163](./FR-163-anomaly-report-release-rationale.md); [FR-103](ix://agent-ix/spec-objects-safety/FR-103), [FR-106](ix://agent-ix/spec-objects-safety/FR-106), [FR-107](ix://agent-ix/spec-objects-safety/FR-107).
- **Downstream**: [FR-199](./FR-199-dossier-risk-management-content.md).
