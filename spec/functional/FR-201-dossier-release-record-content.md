---
id: FR-201
title: "The dossier release record renders the committed snapshot and EA release verdict"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-166"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-167"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-168"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-052"
    type: "references"
  - target: "ix://agent-ix/quire-cli/FR-038"
    type: "references"
---

# FR-201: The dossier release record renders the committed snapshot and EA release verdict

## Description

When `quoin dossier generate` selects `release-record` (document id
`DOS-REL`), the release record SHALL render the supplied committed release
snapshot and Engineering Assurance `release-verify` result as they stand.

## Inputs

- `--snapshot <file>`: a `quoin.release-snapshot/v1` file written by
  `quoin release snapshot` ([FR-166](./FR-166-release-snapshot-command.md),
  [FR-167](./FR-167-release-snapshot-format.md)). The generator reads it
  through a shared read-only `quoin-release-snapshot` library API extracted
  from `rust/crates/quoin-cli/src/release/snapshot.rs`; the CLI and dossier
  use that same parser and closed record model.
- `--release-verify <file>`: one
  `engineering-assurance.release-verify-result/v1` from EA
  [FR-047](ix://agent-ix/engineering-assurance/FR-047).
- The snapshot's FR-168 `change_records` carry the version history that EA
  [FR-052](ix://agent-ix/engineering-assurance/FR-052) supplies. The
  snapshot itself has no release id or name, baseline name or item list.
  Those fields come only from the EA result's `release` and `baseline`
  members when its status is `released`. The dossier does not run either
  source command.

## Outputs

- `dossier/DOS-REL.md`: `# Release record`, `## Release`,
  `## Baseline`, `## Decision`, `## Evidence at release`,
  `## Version history`, then `## Gaps`.
- The release section names the EA `release.id`, `release.name` and
  `release.snapshot_path` when present; the tag is shown only when the
  source EA `approval` member supplies it. The baseline section shows the
  snapshot's `baseline_commit` and, when present, the EA `baseline.name`
  and `baseline.items`.
  The decision section shows source status, approver, meaning, signed-at claim,
  accepted anomalies and, for `not_released`, every EA reason.
- `Evidence at release` shows the snapshot's `coverage`,
  `evidence_freshness`, `open_findings`, `soup` and `anomalies` members in
  their source order and with their source status words.
- The `Decision` section SHALL include these statements verbatim:
  `Approval entries in this export are treated as unbound to the exact bytes of this DOS-REL document because Quoin has not compared their source-record digests to those bytes. They report only the source records and verdicts in their inputs.`
  and `Verifier identity: not provided as a distinct field by the source approval contracts.`

## Behavior

- The document SHALL render the snapshot supplied by `--snapshot` without
  recomputing coverage, anomaly, SOUP or change records from current inputs.
- The document SHALL render `coverage`, `evidence_freshness`,
  `open_findings`, `soup` and `anomalies` from that snapshot under
  `Evidence at release`, without supplementing them from current records.
- When an EA result is `not_released` and has no `release` or `baseline`
  member, the document SHALL show those absent fields as `not stated`
  rather than taking them from current files or inventing them.
- The document SHALL order version-history rows exactly as the snapshot's
  `change_records`, with `Record | Revision | Commit` columns.
- If EA reports `released`, the document SHALL show `released`, approver,
  meaning and `signed_at` byte-equal to the EA result.
- If EA reports `not_released`, the document SHALL show that status and every
  source reason, and SHALL produce `release_not_verified` with those reasons.
  It SHALL NOT describe that status as a successful release.
- Without `--release-verify`, the document SHALL produce
  `no_release_verify_input`; without `--snapshot`, it SHALL produce
  `no_snapshot`. Either condition SHALL make it `incomplete`, not a usage
  error.
- A present but malformed snapshot or verify result SHALL cause source
  failure under FR-191 before any selected document is written.
- The generator SHALL NOT run git, verify a signature, write a tag, choose an
  approver or decide whether release is permitted.
- When exporting this document with an EA `release-verify` result supplied,
  Quoin SHALL append it once to `quire export --approval` after the user's
  `--approval` inputs. Standalone approval verdicts and the nested EA approval
  verdict remain the source inputs they are under Quire-cli FR-038. Quoin SHALL
  NOT compare their record ids or digests to the `DOS-REL` bytes or rewrite
  them. Other standalone approvals in a bundle may refer to other source
  records; Quoin SHALL pass and preserve all supplied entries in order, as
  required by FR-191 and Quire-cli FR-038.
- No Quoin dossier source contract produces an approval specifically for the
  generated `DOS-REL` bytes. EA FR-041 binds a signed tag to a caller-supplied
  change-assurance record id and digest. This requirement does not reinterpret
  that record, compare its digest to DOS-REL, or assign it a digest format.
  The generated statement above treats every included entry as unbound to the
  exact DOS-REL bytes; it does not claim that any input was mismatched against
  a DOS-REL digest.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-201-AC-1 | A `released` fixture shows the EA approver, meaning, signed-at claim, release id/name/snapshot path, baseline name/items and accepted anomalies byte-equal to the EA result; the baseline commit equals the supplied snapshot. | Test |
| FR-201-AC-2 | A `not_released` fixture shows that status, every EA reason and `release_not_verified`; no status cell says `released`. | Test |
| FR-201-AC-3 | Missing `--release-verify` produces `no_release_verify_input`, missing `--snapshot` produces `no_snapshot`, and each leaves the document `incomplete` with command exit 1. | Test |
| FR-201-AC-4 | Version-history rows equal the snapshot's FR-168 `change_records` in source order. | Test |
| FR-201-AC-5 | Changing the evidence store after the snapshot file was written changes no release-record evidence count or history row. | Test |
| FR-201-AC-6 | A malformed source file gives exit 3 naming that source and writes no selected dossier file. | Test |
| FR-201-AC-7 | An unignored integration test using fixture EA results and real export tools yields PDF and DOCX text containing the EA principal and meaning from the approval block, and the generated DOS-REL text states that approval entries are unbound to its exact bytes and that the source contracts do not provide a distinct verifier identity field. | Test |
| FR-201-AC-8 | The generator runs with `git` absent from `PATH`, renders the same bytes, and creates no tag or ref. | Test |
| FR-201-AC-9 | An export containing `DOS-REL`, standalone approval verdicts for other source records, and an EA release result with a nested approval passes every entry to Quire in command-line order, appends the release result once, and renders every verdict entry; the generated DOS-REL text states that all entries are treated as unbound to its exact bytes. | Test |
| FR-201-AC-10 | The `Decision` section contains both the approval-scope and verifier-identity statements exactly as specified in this requirement; the statements occur in the generated Markdown before the final `Gaps` section. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-166](./FR-166-release-snapshot-command.md),
  [FR-167](./FR-167-release-snapshot-format.md),
  [FR-168](./FR-168-release-snapshot-change-records.md), EA
  [FR-047](ix://agent-ix/engineering-assurance/FR-047), EA
  [FR-041](ix://agent-ix/engineering-assurance/FR-041), EA
  [FR-052](ix://agent-ix/engineering-assurance/FR-052), and quire-cli
  [FR-038](ix://agent-ix/quire-cli/FR-038) as extended by PLAT-1761.
- **Downstream**: PLAT-1620 implements this document.

Positive binding to the exact exported DOS-REL bytes is not established by
current Quoin dossier, EA FR-041, or Quire-cli FR-038 contracts. This spec
therefore records the ticket's bound-content acceptance as an upstream
blocker: an upstream-owned producer and an explicit binding contract are
needed before Quoin can truthfully claim or check that relation. In this PR,
the generated document explicitly treats the entries as unbound, leaves
Quire's every-entry rendering intact, and makes no verifier-identity claim.
