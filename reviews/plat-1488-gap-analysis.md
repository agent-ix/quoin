---
id: SR-20261112201002
title: "Gap analysis of quoin PR 833 (FR-140 pack requirements mapping report)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@4e7ad271; FR-140 AC-1..12, FR-135, FR-137, FR-139; rust/crates/quoin-{assurance,cli,core,quire}"
review_set: subset
---

# Gap analysis of quoin PR 833

## Summary

Ticket: PLAT-1488. Plan completion: not assessed. `quoin matrix --repo .` (quoin 0.30.1) lists
FR-140-AC-1..12 as `untagged`. FR-136..139 show the same, because the model reads the
hand-kept `spec/matrix.md` (TM-001) rather than `#[trace]` attributes. That is a pre-existing,
repository-wide state, not a defect in this PR. A manual AC-to-test check finds a
`#[trace("FR-140-AC-n")]` test for every AC (tc_1488_001..012, plus tc_1488_100 for AC-7 at the
core boundary), and each test asserts the AC's oracle. The AC-5 and AC-6 controls do fail their
own oracle. No code path lacks an owning requirement. No stub, no `todo!` and no coverage
inflation were found.

## Verdict

**PASS with one low gap.** Every FR-140 AC has a test that can fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-140's load-failure clause ("naming the set as authority/id@version where known") is met only through the FR-137 `is_pack` refusal, which names the file. The new `pack <name> failed to load` path and the per-set `authority/id@version: <error>` formatting are not reached by any test, since `is_pack` loads the same files first. Add a FakeHost unit test whose `evaluate_clause_sets` fails, and assert the key and exit 2, or drop the unreachable formatting | rust/crates/quoin-core/src/runtime.rs:672-681; rust/crates/quoin-core/src/ops/evidence/discharge.rs:593-595 |
