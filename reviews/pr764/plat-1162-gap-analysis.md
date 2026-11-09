---
id: SR-197
title: "Gap analysis — PLAT-1162 independent evidence gates without apparatus lists (quoin PR 764)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-110-protected-apparatus.md (FR-110-AC-1), spec/matrix.md (TC-1810), rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs, rust/crates/quoin-measurement/tests/fixtures/MP-001-evidence-gate.md"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-110", type: references}
---

## Summary

Ticket: PLAT-1162. PR head: `ac41053`. This review replaces SR-185 on the closed PR #689. It is a planless gap analysis scoped to the PR diff. Plan completion was not assessed. The ticket's four acceptance checks are untrusted text. They were judged against the code and tests, not taken as fact.

- AC-1 (an evidence gate with no apparatus loads): backed by `tc_975_001` and `tc_975_010`. The MP-001 fixture passes the locked EA schema and loads in Quoin.
- AC-2 (the typed rule decides from retained evidence): backed by `tc_975_011`. It covers pass, fail and incomplete for a proportion gate, and `tc_975_010` covers absent evidence.
- AC-3 (procedure or apparatus-edit with no apparatus is refused): the apparatus-edit case is in `tc_975_001`. The procedure case is tested only at observe stage (`tc_1940`).
- AC-4 (validate and verify agree on MP-001): partly backed. See FND-001.

The fixture adds no file paths, digests or version ledger.

## Verdict

**PARTIAL**. AC-1 and AC-2 are backed. The MP-001 end-to-end check stops at absent evidence, and the TC-1810 binding is ambiguous.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Analyze-shaped MP-001 is only verified with no collections (`NoCollections`). Its own `count` / `eq 0` rule is never decided against a retained collection, so ticket AC-4's "validate and verify agree" is not shown for a pass or a reject. Pass and fail are tested only on MP-975's proportion rule. | rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:259-276 |
| FND-002 | medium | The TC-1810 matrix row names `tc_975_004`, `tc_975_010` and `tc_975_011`. Those prefixes also belong to existing tests that TC-1813, TC-1819 and TC-1820 cite (`tc_975_004_a_symlink…`, `tc_975_010_no_apparatus…`, `tc_975_011_a_regenerated…`). So each binding resolves to two tests. | spec/matrix.md:630; rust/crates/quoin-measurement/tests/tc_975_protected_apparatus.rs:207,283,414 |
| FND-003 | low | Ticket AC-3's gate-stage case (a gate with `execution_procedure` and no apparatus) has no test. The code path does not depend on stage, and `tc_1940` covers it at observe stage only. | rust/crates/quoin-measurement/src/plans.rs:193-209 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-110-AC-1 | examined | Backed by tc_975_001, tc_975_004 (new), tc_975_010 (new) |
| FR-108-AC-4 | context_only | tc_975_011 incomplete-population → inconclusive binding correct |
| Ticket AC-1..AC-4 (untrusted) | examined | AC-1/2 backed; AC-3 partial (FND-003); AC-4 partial (FND-001) |
| TC-1810 matrix binding | examined | Ambiguous names (FND-002) |
| Production code with no owning requirement | examined | None; `validate_gate_definition` traces to FR-110 Plan intake |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The FR-108 matrix row still credits PLAT-1162 to `tc_975_010` and `tc_975_011`. After the rename to tc_975_022/023, those names point only at the existing TC-1819/TC-1820 tests, so the row cites the wrong tests. | spec/matrix.md:135 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | ffdd4209 tc_975_027 publishes MP-001 evidence and verifies 0 adverse -> Accept and 1 adverse -> Reject under its own count / eq 0 rule |
| FND-002 | fixed | ffdd4209 renames the new tests to tc_975_022..027 and the TC-1810 row cites `tc_975_001`, `tc_975_022`..`tc_975_027`, which are unique |
| FND-003 | fixed | ffdd4209 tc_975_025 refuses a gate-stage plan with an execution_procedure and no apparatus, naming execution_procedure |
| FND-004 | fixed | 0a0f6d26: the FR-108 row names `tc_975_022` and `tc_975_023` |
