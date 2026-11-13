---
id: SR-20261113052926
title: "Dependency review of licence and ban policy findings (FR-150; FR-041, FR-142, FR-144 amendments)"
type: SpecReview
analysis: dependency
review_set: subset
scope: 'agent-ix/quoin; PR #847 (gh pr diff 847): spec/functional/FR-150-licence-and-ban-policy-findings.md, spec/functional/FR-041-sbom-inventory-evidence.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052926: Dependency review of licence and ban policy findings (FR-150; FR-041, FR-142, FR-144 amendments)

## Summary

Ticket: PLAT-1671. Relationship edges and enablement order: FR-041 reader amendment (enablement) -> FR-150 adapter -> FR-142 join -> FR-144 report. Edges are consistent: FR-150 depends_on FR-041 and extends FR-033/FR-034, FR-142 depends_on FR-150, FR-144 and FR-041 reference it. The NFR-030 edge gap is recorded in the base review. The lead's ruling on FR-150-AC-12 (one minimal hand-written CycloneDX document, clearly marked, with the other fixtures real Syft output) is acceptable in itself and is not vendoring; the fixtures are captured tool output, as FR-034's are. The problem is that the named source cannot produce the licensed cases.

## Verdict

FAIL on one high finding, with one low finding.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | AC-12 says the AC-1 fixtures are Syft output captured from the engineering-assurance FR-039 fixtures, and that the one case none of them holds is an unlicensed component. That is backwards. The FR-039 fixtures (inline in engineering-assurance tests/syft_producer_cli.rs) are a Cargo.lock, a package-lock.json with no `license` fields, a pinned requirements.txt and an OS package database. None of these give Syft a licence, and quoin's existing captured Syft document (tests/fixtures/evidence/syft-cyclonedx-real.json) has no `licenses` on any component. Captured output therefore holds only unlicensed components, and the allowed, denied and not-listed components AC-1 needs have no source. Either the EA fixtures must change (for example an npm lock whose entries carry `license`, which Syft records), which contradicts 'No engineering-assurance change is made', or those cases get hand-written, which breaks the lead's condition that only the unlicensed case is. Fix: name the enablement explicitly, as FR-142-AC-19 does for the Go fixture: an EA FR-039 fixture extension (or a quoin-local scratch project run through Syft) whose captured output holds allowed, denied and not-listed licences; drop the 'no engineering-assurance change' sentence if so; restate AC-12 so the unlicensed component comes from captured output and no hand-written document is needed. | spec/functional/FR-150-licence-and-ban-policy-findings.md:39-40, 120 |
| FND-002 | low | The revision is explicit and justified (ticket outcome: parity with cargo-deny licence and ban findings) and AC-28 tests the new behaviour, so it is not a silent change. But the merged test tagged FR-142-AC-13 (tc_1516_009, rust/crates/quoin-cli/src/soup/advisory/tests.rs) asserts that `rejected` and `banned` diagnostics are set aside, and the code counts them in `set_aside` (soup/advisory.rs:530-545). Until the join changes, main carries a test asserting the opposite of its AC. Fix: say in the ticket or the FR-142 change note that the FR-150 implementation slice rewrites tc_1516_009 and the set_aside count together with the join, in the same PR. | spec/functional/FR-142-advisory-join-onto-soup-records.md:173; rust/crates/quoin-cli/src/soup/advisory/tests.rs:575-597 |
