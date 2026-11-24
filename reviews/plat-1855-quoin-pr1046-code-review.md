---
id: SR-2026112016223475846355015
title: "PLAT-1855 quoin PR 1046 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PR 1046 (rust/crates/quoin-cli/src/class_obligations.rs, rust/crates/quoin-dossier/src/{architecture,trace_matrix}.rs, rust/crates/quoin-assurance/tests/tc_1080_matrix.rs)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
  - target: "ix://agent-ix/quoin/FR-193"
    type: references
  - target: "ix://agent-ix/quoin/FR-194"
    type: references
---

# PLAT-1855 quoin PR 1046 code-review

## Summary

Ticket: PLAT-1855. Code review with the Rust lane (rust-review + rust-style) folded in, scoped to `git diff origin/main...HEAD` (head f9a0d3ca, base 2900ad56).

`design_status` and `design_rows` are deleted and `design-trace` goes through `relation_status`. For an unpinned entry the new path reads the same rows as the old one: it starts from the same relations, keeps the same `row_is_of` filter and the same `fold_rows(rows, &[])` fold. It keeps the same two `not-available` reasons ("design join has no relations" for an absent or empty join, and "requirement has no design relation row" for no row). The only output change is the new `missing` list on an `unmet` row, as FR-161 states. A pinned entry filters relations by `name` before reading rows, so `missing` can only hold its own relation. `missing` follows join order because relations are iterated in join order. A relation is pushed once even when the requirement has several `uncovered` rows in it. `parse_activities` refuses `element_archetypes`, a method and a blank `relation` on `design-trace`. `check_relations_declared` already covers every kind that carries `relation`, so an undeclared pinned relation is refused with no new code.

Both dossier generators now loop over blocks and emit one `no_design_element` gap per `uncovered` block, carrying its `check` token. The detail "design relation check `<check>` found no element" never says the requirement has no design element. `trace_matrix.rs` keeps PR 1040's `safety_class_gap` call in front of the loop. The gap sort in `gap.rs` is stable, so two gaps of one requirement keep block order.

The `unnamed` fallback for a block with no `check` matches how these generators already read optional join fields: `text()` falls back to an empty value, and `array()` to empty. It still records the gap, so nothing is dropped. FR-165 says quire-rs always supplies `check`, so the fallback only covers malformed input. It is acceptable and is not a finding.

The old heading "Requirements without a design element" has no other consumer: no golden, snapshot or spec names it. The only hits are the updated unit test and an old review file.

Rust idiom: no `unwrap`, `expect`, indexing or panic in production code. `Kind` is still matched exhaustively in `build_rows`. Ordering is deterministic, with no needless clones beyond one `String` per gap. No pin, SHA or count test was added.

No other open PR touches these files. PR 1041 changes `gap.rs`, and PR 1039 changes `quoin-quire/src/design_join.rs` but not the files of this diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The architecture document's "Requirements with an uncovered design relation" list is no longer in dossier order. The old code listed `sorted(&join.without_design)`, which uses `dossier_cmp`. The new code iterates `BTreeMap<String, Vec<String>>` directly, which is byte order, so `FR-10` is listed before `FR-2`. FR-191 Ordering requires ids in dossier order, and every other list in this file still uses `sorted`/`sorted_pairs`. The emitted gaps are re-sorted later by `sort_gaps`, but the body list is not. No test has two requirements whose byte order and dossier order differ. Fix: iterate the keys through `dossier_cmp` and add a test with `FR-2` and `FR-10`. | rust/crates/quoin-dossier/src/architecture.rs:329-341 |
| FND-002 | low | The `activity.kind == Kind::DesignTrace` guard on `missing` in `relation_status` has no test that fails without it. Removing the guard adds `missing` to every `unmet` `soup-requirements`, `segregation` and `detailed-design` row. The FR-161-AC-17 test asserts `missing` on `activities[0]` (design-trace) only, and the AC-10 and AC-11 tests assert statuses only. FR-161 defines `missing` for `unmet` `design-trace` rows only. Assert that the `unmet` soup row in `design_trace_soup_and_segregation_entries_report_one_row_each` carries no `missing`. | rust/crates/quoin-cli/src/class_obligations.rs:405-410; rust/crates/quoin-cli/src/class_obligations.rs:1071-1074 |

## Gates

All runs used `/home/peter/dev/worktrees/locked-build.sh` with debug=0, incremental=0 and the shared CARGO_HOME, on head f9a0d3ca:

- `cargo test -p quoin-cli --bin quoin class_obligations`: 13 passed.
- `cargo test -p quoin-cli --test tc_160_class_policy_command`: 14 passed.
- `cargo test -p quoin-dossier --lib`: 129 passed.
- `cargo test -p quoin-assurance --test tc_1080_matrix`: 30 passed.
- `cargo clippy -p quoin-cli -p quoin-dossier --lib --bins`: no warning in any file of the diff. The only warnings are 13 pre-existing ones in `quoin-evidence`.
- `cargo fmt --all --check`: clean.

For FND-002, I grepped every `missing` assertion in the class_obligations unit tests and in tc_160. None asserts that a non-design-trace row has no `missing`, so the mutant survives. I did not mutate or build the branch.

## Verdict

CONDITIONAL. The design-trace routing, the pinned reading, the refusals and the per-block dossier gaps are implemented as FR-161, FR-193 and FR-194 state. The labels are truthful: no gap claims a requirement has no design element. FND-001 is an ordering regression in the architecture document body and should be fixed in this PR. FND-002 is a missing negative assertion.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new test `uncovered_requirements_are_listed_in_dossier_order` is tagged `FR-191-AC-14`. That criterion is about the `Gaps` section: two bullets in gap order, the same order in `--json`, and `Gaps: none`. Dossier order of requirement ids is `FR-191-AC-15` ("lists them in dossier order"). Retag the test `FR-191-AC-15`. | rust/crates/quoin-dossier/src/architecture.rs:726-741 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | a4b8680f: `findings_sections` collects `join.uncovered.keys()` and sorts them with `dossier_cmp`. `uncovered_requirements_are_listed_in_dossier_order` renders FR-10 and FR-2. Mutation run: deleting the `sort_by` line makes that test fail (architecture.rs:733). |
| FND-002 | still-open | At a4b8680f there is no test named `only_design_trace_rows_list_missing_relations`, and no test asserts that a non-design-trace row lacks `missing`. The fix commit's message claims "missing only on design-trace", but the commit does not touch `relation_status` or its tests. Mutation run: `if value == "unmet" && activity.kind == Kind::DesignTrace` changed to `if value == "unmet"` leaves `class_obligations` passing (13 passed, mutant survives). Add the negative assertion. |
| FND-002 | fixed | FR-1 has an uncovered S row. The test checks that design trace lists S and the unmet soup activity omits the missing field. FR-3 makes the same check for unmet segregation. |
| FND-003 | fixed | The dossier-order test now carries FR-191-AC-15. The computed matrix binds this test to the ordering criterion, and its assertion still requires FR-2 before FR-10. |

### Disposition pass 2 evidence

Reviewed commit `de205cd1b04584e11bc8e0518da8f46b1b419d5a`.

FND-002 after_excerpt (rust/crates/quoin-cli/src/class_obligations.rs, lines 1091–1092)

```rust
        assert_eq!(activities(0)[0]["missing"], json!(["S"]));
        assert!(activities(0)[1].get("missing").is_none());
```

FND-003 after_excerpt (rust/crates/quoin-dossier/src/architecture.rs, lines 726–728)

```rust
    #[trace("FR-191-AC-15")]
    #[test]
    fn uncovered_requirements_are_listed_in_dossier_order() {
```
