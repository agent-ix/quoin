---
id: SR-17095
title: "Integrity review of PLAT-1839 FR-211"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-211-module-lag-warning.md, spec/functional/index.md, spec/spec.md; PR agent-ix/quoin#1027"
review_set: subset
---

## Summary

Ticket: PLAT-1839. Integrity review of the new FR and its index entries, measured against current `origin/main` (86313ad5) as well as the branch base (c29ac63b).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Requirement id collision: `origin/main` now carries `spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md` with `id: FR-211` (merged in #1026 after this branch's base). Merging gives two FR-211 documents, and this PR's `#[trace("FR-211-AC-1".."AC-3")]` tags and `#[implements("FR-211")]` would bind to the discharge requirement's criteria. The new FR needs the next free id (FR-212 at time of review) and all tags renumbered. | spec/functional/FR-211-module-lag-warning.md:2 |
| FND-002 | medium | The Behavior rule that a module "has a ref that is not a semantic version is not reported" leaves the stale-install case silent when the install is unversioned: earlier default manifests pinned some modules to a default branch (e.g. spec-artifacts-app before 565143fd), lazy reconcile never replaces such an install, and doctor stays quiet although the content is not the pinned release. An installed ref that differs from a release-tag pin and cannot be compared should still warn (as "not at the pinned release"). | spec/functional/FR-211-module-lag-warning.md:36-37, rust/crates/quoin-modules/src/lag.rs:43-45 |
| FND-003 | low | `spec/functional/index.md` inserts FR-211 between FR-017 and FR-018 in an otherwise id-ordered list. | spec/functional/index.md:28 |

## Verdict

Not mergeable until the id collision is resolved. Relationships (traces_to StR-005, references FR-017) resolve and the FR is otherwise structurally complete.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Deleting the pin-tracking test is right, but FR-016-AC-2 and AC-4 are left live with Verification "None (... not asserted by a test)" (matrix: untagged, method None), an AC nothing can fail kept as informational text. Withdraw them with a CR note as AC-3 was, or restate a checkable behaviour; the dropped assertion that every default entry is a Git source with an explicit ref was real behaviour FR-212 depends on (a non-version pin silently disables the lag check per FR-212-AC-4) and belongs in `the_shipped_default_module_manifest_is_valid`. | spec/functional/FR-016-default-modules-schema.md:85-86, rust/crates/quoin-cli/src/module.rs:559-563 |

## Dispositions

Round 1, head 35474e19.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Renumbered to FR-212. origin/main (86313ad5) has no FR-212, no other open quoin PR adds FR-212, and no FR-211 reference remains in rust/ or the new FR. |
| FND-002 | fixed | `LagKind::Incomparable` warns for a branch or absent installed ref against a version pin (FR-212-AC-3), with unit and two integration tests. |
| FND-003 | fixed | FR-212 is appended after FR-211 in index.md and spec.md. |
| FND-004 | fixed | Round 2 (20bafb66): FR-016-AC-2 and AC-4 are withdrawn by a CR note (ids not reused); new FR-016-AC-6 "Every default module entry is a Git source with a non-empty explicit `source.ref`" is tagged in `quoin matrix` and `the_shipped_default_module_manifest_is_valid` asserts it per entry without asserting any version. |
