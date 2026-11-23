---
id: SR-20261123202510
title: "Base review of release snapshot risk-completeness and anomaly-gate open findings"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-166-release-snapshot-command.md, spec/functional/FR-167-release-snapshot-format.md"
review_set: subset
---

## Summary

Ticket: PLAT-1808 (PR agent-ix/quoin#1033). Base checklist review of the FR-166 and FR-167 changes that add
risk-completeness findings (FR-170 tokens, built as `quoin risk-summary` builds them) and FR-164 anomaly-gate
failures to the release snapshot's `open_findings`, checked against FR-115, FR-144, FR-162, FR-164, FR-168, FR-169,
FR-170, FR-201, NFR-036 and the code in `rust/crates/quoin-cli/src/release.rs`, `release/snapshot.rs`,
`quoin-risk-advice/src/lib.rs`, `quoin-anomaly/src/report.rs` and `quoin-dossier/src/release_snapshot.rs`.

The design holds: the closed item shape is kept, `kind` stays an open string in the dossier reader
(`OpenFinding.kind: String`), so old v1 snapshots still parse and FR-201 renders the new items without change; the
dedupe key, `obligation`/`summary` mapping, sort order (`OpenFinding::order_key`) and the exit-2 refusal are
consistent with FR-169/FR-170; a repository with no risk records is not broken, because the quire reader raises
"type declared by no loaded module" only for a document of a checked type that exists; no severity or acceptability
is assigned; no ticket citation, paywalled text, digest or pin is added. One contradiction blocks merge: FR-167
forbids the word "accepted" in the snapshot, while three FR-170 tokens it now requires as `kind` contain it. The
anomaly-gate `path` mapping also yields an absolute path under an absolute `--repo`, against NFR-036.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | FR-167 Behavior "SHALL NOT include ... the word accepted" (and FR-167-AC-7, whose test scans the whole body for `accepted`) contradicts the new mapping that writes FR-170 tokens `accepted-outside-acceptable-region`, `benefit-risk-not-accepted` and `not-accepted-inside-acceptable-region` as `kind`; FR-167-AC-17 requires such an item while FR-167-AC-19 requires no acceptance word. Add FR-169's carve-out (the stem appears only in a finding token) to FR-167's behavior, AC-7 and AC-19. | FR-167, FR-167-AC-7, FR-167-AC-17, FR-167-AC-19, FR-169 |
| FND-002 | medium | The anomaly-gate item's `path` is "the failure's path"; FR-164's gate path is the document path under the scope as given (`shown()` strips only `./`), so `--repo /abs/checkout` writes an absolute, host-dependent path into the committed snapshot, which NFR-036 forbids (only repository-relative `path` members are allowed) and the existing determinism test rejects. State `path` as repository-relative. | FR-167, FR-167-AC-18, NFR-036, FR-164 |
| FND-003 | low | "An `open_findings` list holding none of these three sources states that no finding exists in them" is untrue for `quoin.release-snapshot/v1` files written before this change, which carry the same protocol; and "A snapshot is regenerated to carry the added findings" can be read as rewriting a snapshot already committed in a tagged release. Scope the first sentence to snapshots from a Quoin version that computes these sources (readable from `tool_identity.version`) and say the next snapshot written carries them. | FR-167 |

## Verdict

Not mergeable until FND-001 is fixed; FND-002 should be fixed in the same round. The rest of the change is sound
and consistent with the code it describes.
