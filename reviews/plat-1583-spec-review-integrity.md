---
id: SR-17083
title: Integrity review of quoin PR 872 (PLAT-1583, FR-175)
type: SpecReview
analysis: integrity
scope: agent-ix/quoin@5fbc1576 (PR 872); spec/functional/FR-175-anomaly-check-fix-verification-evidence.md, spec/functional/index.md
review_set: subset
date: '2026-11-13'
---

# Integrity review of quoin PR 872 (PLAT-1583, FR-175)

## Summary

Ticket: PLAT-1583. Completeness and consistency of FR-175 against FR-136 (record reader, diagnostics, no re-reporting of references), FR-030 (run selection, unreadable files) and the spec-artifacts-process fixtures it names.

## Verdict

Changes requested. Every fixture FR-175 names exists in the sibling checkout with the content the ACs assume: test-only TC-001, criterion-only FR-001-AC-1, closed-fixed with both rows, resolved-fixed, and dangling TC-999. TC-001 is a `Test ID` of TM-001 in the corpus. The table codes match the bullets. Three gaps remain: the record reader the check needs is unstated, a skipped dangling row is handled inconsistently, and run selection after an unreadable run file is unspecified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The check needs the `## Fix Verification` rows and needs to decide whether a Target resolves, including to a TestMatrix `Test ID`. The FR-136 CR note (PLAT-1415) limits the temporary reader in `quoin-anomaly/src/record.rs` to eight named keys, and `verification` is not one of them. `refs.rs` `resolvable_ids` knows FR/NFR, their criteria and hazards, but not test cases. FR-175 widens that stand-in and Quoin's own reference resolution without saying so. It also leaves open whether a malformed Fix Verification table (`invalid/verification-bad-headers.md`) becomes a mapping error, and so `anomaly.no-record`. That would change FR-136 output for fixtures that report a record today. State the widening, as FR-162 did, and say whether a bad verification table is a mapping error. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:44-45, 100 |
| FND-002 | medium | `invalid/verification-dangling-test.md` has a single Test row, TC-999, that resolves to nothing. Line 44 skips that row. Line 77 then sees a table with no evaluated Test row and no passing Criterion row, so a literal reading emits `anomaly.fix-no-passing-link`. AC-8 requires none of the three codes. Whether a skipped row still counts as 'a Test row' for line 77 is never stated, and two implementers would differ. State it, for example: 'a document whose rows all resolve to nothing yields none of the three'. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:44-45, 77-79, 100 |
| FND-003 | medium | FR-030-AC-13 skips and reports an unreadable run file instead of failing. If the newest run of a suite is unreadable, 'the newest run of each suite' becomes the next-older readable run. An older passing run then makes the Target `passing`, which line 60 forbids. FR-175 does not define the result when a suite's newest run file cannot be read. Say that such a suite's entries read as `missing` (or as `failing`) and are never replaced by an older run. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:52-53, 60 |
| FND-004 | low | `anomaly.fix-test-failing` is also emitted for a failing Criterion row (AC-6 names the criterion id), so the code name says 'test' where the row is a criterion. Rename it, for example to `anomaly.fix-target-failing`, before the code ships. | spec/functional/FR-175-anomaly-check-fix-verification-evidence.md:68, 72 |
