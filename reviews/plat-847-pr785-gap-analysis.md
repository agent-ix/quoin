---
id: SR-20261103118153
title: "Gap analysis of quoin PR 785 (why the retag binds but does not back)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@dc1e1e7; rust/crates/quoin-measurement, rust/crates/quoin-measurement-graph, spec/matrix.md"
review_set: subset
---

# Gap analysis of quoin PR 785

## Summary

Ticket: PLAT-847. Planless, scoped to the criteria the PR's markers name (144 distinct ids
over 261 markers) and the matrix rows that reference them. Evidence: `quire coverage --json`
(quire 0.39.0, engine 0.55.0) on the merge base and on dc1e1e7.

| Measure | Base | Head |
| --- | --- | --- |
| rust bound symbols | 88 | 349 |
| backed rows (whole repo) | 1 | 2 |
| `trace-id-outside-marker` in these crates | 545 | 242 |
| `criterion-tagged-outside-row` in these crates | 16 | 300 |

The one new backed row is `StR-007-VC-1`. A stakeholder validation criterion is backed
directly by its tag, while acceptance criteria and matrix rows are not.

Why backed rows hardly moved: the module backs a `spec/matrix.md` "traces-to" row (for
example `TC-1003 -> FR-044-AC-1`) only when a test carries **the row's own TC id**. The
`verification` rows in each FR (`FR-044-AC-10 -> TC-1948, TC-1949, TC-1952`) back through
those TC rows too. The retag carried the ids that were written, which are almost all AC ids.
So every newly bound test now produces `criterion-tagged-outside-row`, and quire names the
missing TC id in each one. For example: "criterion `FR-100-AC-8` is tagged on
`tc_476_020_there_is_exactly_one_canonical_json_writer` ... expected a same-test tag for
one of `TC-1640`. This tag does not provide backing". All 300 have this form.

Plan completion: not assessed

## Verdict

**CONDITIONAL.** The retag is a correct and necessary precondition: before it, those tests
were not bound at all. It does not back the rows. Two gaps block backing, and neither is a
fidelity defect in the diff. Semantic review was skipped (not opted in).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | In these crates, quire reports `criterion-tagged-outside-row` for 300 acceptance-criterion tags on tests that carry no TC id. Each diagnostic and names the TC row id the matrix cites for that test (e.g. TC-1640 for tc_476_020). Without that TC id in the marker, no matrix row is backed. The diagnostic gives the exact id, so adding it is mechanical. It is the step that makes the retag count toward `coverage.backed` | rust/crates/quoin-measurement-graph/tests/tc_476_boundary.rs:94, rust/crates/quoin-measurement/tests/tc_479_fr044_fr045_criteria.rs:301 |
| FND-002 | medium | A blank line at spec/matrix.md:1373 splits the "Test Case Summary" table. Rows TC-1940 onward (lines 1374-1439) have no header row, so none are minted. The last minted test-case row is at line 1372. The TC ids this PR does carry (TC-1941, TC-1948 to TC-1962, TC-1983) therefore bind to rows that do not exist. This is pre-existing and outside the diff | spec/matrix.md:1373 |

## Coverage

- Criteria in scope: 144 distinct ids on the PR's markers. 127 are minted targets. The 17
  that are not are FR-107-CON-3 and the 16 TC ids stranded by FND-002.
- Untagged criteria introduced by the PR: none. No tag was dropped (see the code-review
  artifact's fidelity check).
- Reverse gap and stubs: not applicable. The diff adds no production code.
- Semantic review: skipped.
- Plan completion: not assessed

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | 32 markers (on tests that existed before this PR) now back 4 rows that `spec/matrix.md` marks 🚧, and those tests do not assert the rows. TC-1634 (re-serialising every store is byte-identical) is backed by 11 tests such as the tc_471 record-id grammar and base64 vectors. TC-1636 (Rust reports match the retained TypeScript) is backed by 14, such as the tc_469 wire-enum round trip and tc_477 path refusals. TC-1638 (three-part retention answer) is backed by the 3 agent_eval scenario/report tests. TC-1667 (NFR-025-AC-3) is backed by 4 tc_468/tc_472 tests. The FR-100 and NFR-025 ACs each have one TC, so the TC id was derived from loose AC tags that were already there. That turns those tags into false backing in `coverage.backed`. Drop these TC ids, and preferably the AC tags too | rust/crates/quoin-measurement/tests/tc_471_intake.rs:35, rust/crates/quoin-measurement/tests/tc_469_wire_enums.rs:83, rust/crates/quoin-measurement/src/intervention/agent_eval/produce.rs:358, rust/crates/quoin-measurement/tests/tc_468_boundary.rs:90 |
| FND-004 | low | FR-114's verification column lists TC-1947 as verifying FR-114-AC-3. Matrix row TC-1947's criterion cell lists only FR-114-AC-2 and FR-114-AC-4, so `tc_1947_two_member_authored_origins_are_recorded` keeps a `criterion-tagged-outside-row` for FR-114-AC-3. The fix is to add FR-114-AC-3 to row TC-1947, not to add TC-1983 | spec/matrix.md:1380, spec/functional/FR-114-generic-measurement-campaign.md:45 |
| FND-005 | low | No matrix row cites `tc_969_003_a_publish_whose_every_local_artifact_is_readable_is_admitted`. Row TC-1734 states the matching property ("a name that exists locally never appears there"). Cite the test in TC-1734 and tag it `TC-1734` | rust/crates/quoin-measurement/tests/tc_479_fr044_fr045_criteria.rs:1067, spec/matrix.md:565 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
