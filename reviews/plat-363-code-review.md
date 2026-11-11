---
id: SR-20261111175655
title: "Code review of quoin PR 803 (assurance case strategy nodes and bound-leaf evidence)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@83c7baf0; rust/crates/quoin-assurance/src/case.rs, rust/crates/quoin-assurance/src/matrix.rs, rust/crates/quoin-assurance/src/render.rs, rust/crates/quoin-assurance/tests/case_graph.rs, rust/crates/quoin-assurance/tests/case_rendering.rs, rust/crates/quoin-assurance/tests/common/fixtures.rs, rust/crates/quoin-cli/src/assurance.rs"
review_set: subset
---

# Code review of quoin PR 803

## Summary

Ticket: PLAT-363. Scoped to `git diff 4c174e58..83c7baf0` (8 files, +428/-50). The PR adds
`NodeKind::Strategy` nodes naming edge verbs between a goal and its sub-claims, dedups a child
that declares several verbs toward one parent, and gives bound solution leaves an `evidence`
block (bound suites, newest run/scan timestamp, verdict). The `rust-review` lane is folded in,
with the repo's `.claude/skills/rust-style` loaded. Tools: quoin 0.30.0.

Checks run:

- `cargo test -p quoin-assurance --test case_graph --test case_rendering`: 17 passed.
- `cargo clippy -p quoin-assurance --tests -D warnings` is clean, and `cargo fmt --check` is clean.
- quoin-core boundary tests (`tc_447_assurance_boundary`) were not run. quoin-semantic's
  build.rs needs `node_modules` in the coder's worktree, which this review kept read-only.
  `rust/goldens/quoin-assurance/cases.json` holds inputs only, so strategy nodes do not
  invalidate it.
- The verdict path was traced against `quoin-auditor/src/audit/mod.rs:115-175` and
  `matrix.rs::status`.
- Timestamp order was traced against the store (`quoin-evidence/src/store/records.rs:161`)
  and the RFC 3339 reader (`quoin_store::instant`).
- The mermaid shape `[/"…"/]` and label quoting were checked against `render.rs::label`.

## Verdict

**NOT MERGEABLE. One high finding and three medium.** Strategy nodes, the deduplication and
mermaid output are correct and deterministic. Strategy statements are built from `BTreeMap`s,
the parallelogram shape is valid, labels stay quoted, and `;` is sanitised.

The verdict does come from audit findings through `matrix::bucket`. However, `fresh` is
inferred from the absence of a finding. The auditor explicitly does not make that inference,
so the view can report `fresh` where the FR-115 matrix reports `undischarged`.

The `newest` value is a lexical maximum over caller-supplied RFC 3339 text, which misorders
fractional seconds and offsets.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Verdict `fresh` is inferred from "no finding", not from the auditor's `healthy` list. A bound obligation whose ladder stops early, such as one bound only to scans, has no finding and is not healthy. An obligation with an unevaluated check is the same. Both render `verdict fresh` while the FR-115 matrix says `undischarged`. The CLI never passes `report.healthy` or `report.unevaluated` to `build_case`. Fix: pass both, make `Fresh` require membership of `healthy`, and reuse `matrix::status` rather than re-matching bucket numbers | rust/crates/quoin-assurance/src/case.rs:642-665, rust/crates/quoin-cli/src/assurance.rs:187 |
| FND-002 | medium | `newest` compares timestamp strings byte-wise. RFC 3339 allows fractional seconds and offsets, and the store does not normalise them. `2026-11-09T00:00:00.500Z` sorts before `2026-11-09T00:00:00Z`, because `.` is less than `Z`, so the earlier instant wins; `+01:00` offsets misorder the same way. Compare by `quoin_store::instant::Instant::parse(_, Rfc3339).epoch_millis()`, which is already a dependency (see `argument/instant.rs`), with a deterministic tiebreak | rust/crates/quoin-assurance/src/case.rs:571, rust/crates/quoin-assurance/src/case.rs:651 |
| FND-003 | medium | The new behaviour's branches are untested: a child declaring two verbs toward one parent (the duplicate-child fix, "appears once, under both"), the `suspect` verdict, a leaf with another finding kind (no verdict), and a bound suite with no run or scan (`no run or scan recorded`) | rust/crates/quoin-assurance/tests/case_rendering.rs:224-327 |
| FND-004 | medium | Leaves with any other finding render `verdict not stated`, but the auditor did state something: FR-115 calls these `undischarged`. After the FND-001 fix, an unevaluated leaf would render `not stated` with no `because` line at all. Use the matrix's closed vocabulary (bound/stale/suspect/undischarged) or render `undischarged` | rust/crates/quoin-assurance/src/render.rs:247, rust/crates/quoin-assurance/src/case.rs:657-660 |
| FND-005 | low | In mermaid, the label sanitiser turns the strategy's `;` verb separator into `,`, giving `'refines': FR-003, 'traces_to': FR-001, FR-002`. The verb-to-id grouping becomes ambiguous. Use a separator the sanitiser keeps (e.g. ` \| ` or ` / `) in the statement | rust/crates/quoin-assurance/src/case.rs:613 |

## Examined scope

```yaml
clean: false
scope:
- {id: case.rs/strategy_for, path: rust/crates/quoin-assurance/src/case.rs, role: examined, excerpt: "strategy node per decomposed goal; verbs grouped by BTreeMap; open if any sub-node open"}
- {id: case.rs/children-dedup, path: rust/crates/quoin-assurance/src/case.rs, role: examined, excerpt: "parent -> child -> BTreeSet<verb>; a child is one sub-claim per parent"}
- {id: case.rs/evidence_for, path: rust/crates/quoin-assurance/src/case.rs, role: examined, excerpt: "verdict = min(bucket(kind)); None => Fresh"}
- {id: render.rs/walk, path: rust/crates/quoin-assurance/src/render.rs, role: examined, excerpt: "NodeKind::Strategy => (\"[/\", \"/]\")"}
- {id: matrix.rs/bucket, path: rust/crates/quoin-assurance/src/matrix.rs, role: examined, excerpt: "pub(crate) fn bucket(kind: &str) -> u8"}
- {id: cli/assurance.rs/observations, path: rust/crates/quoin-cli/src/assurance.rs, role: examined, excerpt: "observations from store runs/scans: suite + timestamp"}
```
