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
  - target: "ix://agent-ix/engineering-assurance/FR-052"
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
  through the parser in `rust/crates/quoin-cli/src/release/snapshot.rs`,
  exposed as a shared read-only library API for the dossier.
- `--release-verify <file>`: one
  `engineering-assurance.release-verify-result/v1` from EA
  [FR-047](ix://agent-ix/engineering-assurance/FR-047).
- The snapshot's FR-168 `change_records` already carry the version history
  that EA [FR-052](ix://agent-ix/engineering-assurance/FR-052) supplies.
  The dossier does not run either source command.

## Outputs

- `dossier/DOS-REL.md`: `# Release record`, `## Release`,
  `## Baseline`, `## Decision`, `## Evidence at release`,
  `## Version history`, then `## Gaps`.
- The release section names the source release id, name, tag and snapshot
  path when present. The baseline section lists its name, commit and items.
  The decision section shows source status, approver, meaning, signed-at claim,
  accepted anomalies and, for `not_released`, every EA reason.

## Behavior

- The document SHALL render the snapshot supplied by `--snapshot` without
  recomputing coverage, anomaly, SOUP or change records from current inputs.
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
- When exporting this document with an EA result supplied, the command SHALL
  pass that result directly to `quire export --approval`. Quire-cli FR-038
  needs the source-contract extension in PLAT-1761 to accept the EA release
  result; the dossier SHALL NOT write an intermediate approval verdict file.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-201-AC-1 | A `released` fixture shows the EA approver, meaning, signed-at claim and accepted anomalies byte-equal to the source, and the baseline items equal the supplied snapshot. | Test |
| FR-201-AC-2 | A `not_released` fixture shows that status, every EA reason and `release_not_verified`; no status cell says `released`. | Test |
| FR-201-AC-3 | Missing `--release-verify` produces `no_release_verify_input`, missing `--snapshot` produces `no_snapshot`, and each leaves the document `incomplete` with command exit 1. | Test |
| FR-201-AC-4 | Version-history rows equal the snapshot's FR-168 `change_records` in source order. | Test |
| FR-201-AC-5 | Changing the evidence store after the snapshot file was written changes no release-record evidence count or history row. | Test |
| FR-201-AC-6 | A malformed source file gives exit 3 naming that source and writes no selected dossier file. | Test |
| FR-201-AC-7 | An ignored real-tool export test yields PDF and DOCX text containing the EA principal and meaning from the approval block, through PLAT-1761's public input contract. | Test |
| FR-201-AC-8 | The generator runs with `git` absent from `PATH`, renders the same bytes, and creates no tag or ref. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md),
  [FR-166](./FR-166-release-snapshot-command.md),
  [FR-167](./FR-167-release-snapshot-format.md),
  [FR-168](./FR-168-release-snapshot-change-records.md), EA
  [FR-047](ix://agent-ix/engineering-assurance/FR-047), EA
  [FR-052](ix://agent-ix/engineering-assurance/FR-052), and quire-cli
  [FR-038](ix://agent-ix/quire-cli/FR-038) as extended by PLAT-1761.
- **Downstream**: PLAT-1620 implements this document.
