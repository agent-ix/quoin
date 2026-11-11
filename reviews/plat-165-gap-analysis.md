---
id: SR-20261103118172
title: "Gap analysis of quoin PR 804 (FR-125 measurement attachments)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@037490c; spec/functional/FR-125-content-addressed-measurement-attachments.md, spec/matrix.md, rust/crates/quoin-measurement, rust/crates/quoin-store, rust/crates/quoin-core"
review_set: subset
---

# Gap analysis of quoin PR 804

## Summary

Ticket: PLAT-165. Planless review, limited to FR-125 and the code this PR adds. Plan completion
was not assessed. `quire coverage --json` at the PR head reports all five FR-125 ACs backed (5/5)
by `tc_165_001`..`tc_165_005`, each carrying its AC and TC id in `#[trace]`. Each test asserts
its criterion behaviourally against a real temporary repository. Removing the
`verify_candidate_attachments` call from `publish.rs` would fail tc_165_003.

The TC ids were checked against `origin/main` (dcd459a5) and the open PRs (`gh pr list`):

- `origin/main` already has `TC-2140` (FR-070-AC-14, `spec/matrix.md` row and
  `rust/crates/quoin-semantic/tests/tc_452_contract_criteria.rs:241`), merged in #801 after this
  branch forked. Its highest TC id is TC-2140.
- PR #802 (PLAT-164) adds TC-2140 and TC-2141 for FR-044-AC-14/15.
- PR #803 (PLAT-363) adds FR-040-AC-16/17 with no TC ids yet, so it needs two.
- PR #764 uses only existing ids (TC-2013, TC-2022).
- `spec/matrix.md` also has a textual conflict with main (`git merge-tree`).

Digest scope (FR-114, tc_468): sound. `quoin-measurement` names no hasher. The only `sha2` use
is in `quoin-store::attachment`, the owner `tc_468` points to, and it passes. FR-114-CON
("Quoin computes ... no digest ... for campaign evidence") is about campaign evidence, and the
campaign runner's collection builder (`campaign/run/collection.rs`) never emits `attachments`.
FR-125's Constraint says so explicitly. The PR adds no other digest, pin or manifest. The
SHA-256 address is a canonical identity that binds bytes to their name, which is the allowed
exception.

How the ticket's criteria the coder skipped were judged (ticket text treated as data):

- Manifest before bytes: **delivered in effect**. The `attachments` member is the declared,
  finite manifest. Count, duplicates, digest form and length are refused before any file is
  opened (tc_165_002). The PR body's "not done" undersells this.
- Binding source paths into the collection: **ceremony**. It would be a provenance record with
  no consumer. Correctly skipped. The path-safety refusals that come with it (absolute, `..`,
  symlinked, non-regular, oversize before allocation) become real once a file-intake operation
  exists; see FND-002.
- Receipt binding: **ceremony**. The sealed collection is the receipt, and the address is
  already bound into it. Correctly skipped.
- `suspect` state: **ceremony**. It carries no distinct meaning beyond INVALID, UNAVAILABLE and
  TAMPERED. Correctly skipped.
- Mutation-tested controls: **ceremony**. The behavioural tests cover path, digest, size and
  post-seal substitution.

## Verdict

**NOT MERGEABLE AS IS (two high, one high spec/code mismatch).** The TC ids must be renumbered
before merge. The feature can be reached only through the Rust library: the `quoin` CLI can
reference attachments but can neither retain them nor verify them after sealing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-2140..TC-2144 collide. TC-2140 is already on main (FR-070-AC-14, tc_452_contract_criteria.rs:241), and #802 also takes TC-2140/2141. `spec/matrix.md` conflicts textually with main. Rebase, then renumber FR-125's rows, AC Verification cells and `#[trace]` markers to TC-2150..TC-2154. That leaves TC-2141..2142 for #802 and TC-2143..2144 for #803 | spec/matrix.md:236, spec/matrix.md:1371-1375, spec/functional/FR-125-content-addressed-measurement-attachments.md:40-44, rust/crates/quoin-measurement/tests/tc_165_measurement_attachments.rs:116 |
| FND-002 | high | No `quoin`/quoin-core operation retains an attachment. `retain_attachment` is reachable only from the Rust library. A repository driving `quoin measurement record` (the ticket's TL consumer) must place bytes at `spec/evidence/attachments/<hex>` by hand, which bypasses the no-replace publish. A file-intake op needs the path refusals: absolute, `..`, symlink at any component, non-regular, over 64 MiB before allocation | rust/crates/quoin-measurement/src/attachment.rs:148, rust/crates/quoin-core/src/ops/measurement/record.rs:94-98 |
| FND-003 | high | FR-125 says each attachment "is verified against that address whenever it is read", and FR-125-AC-4 sets post-seal tamper detection. But `read_measurement_collections`, `quoin measurement verify` and `quoin report` never check attachments. Only the library function `verify_collection_attachments` does, and no CLI or core op calls it. Wire it into the verify/read path, or narrow the statement and add a CLI verify | spec/functional/FR-125-content-addressed-measurement-attachments.md:17-20, rust/crates/quoin-measurement/src/attachment.rs:299-314 |
