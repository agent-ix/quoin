---
id: SR-2026112016223475846355014
title: "PLAT-1855 quoin PR 1042 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR 1042 (FR-161-AC-12, FR-161-AC-15..17, FR-165-AC-20) against rust/crates/quoin-cli/src/class_obligations.rs and its tests"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
  - target: "ix://agent-ix/quoin/FR-165"
    type: references
---

# PLAT-1855 quoin PR 1042 gap-analysis

## Summary

Ticket: PLAT-1855. Spec-only PR; no cargo or make ci run. `quoin matrix --repo . --json` (quoin 0.36.8) on head d561a059: FR-161-AC-15, AC-16, AC-17 and FR-165-AC-20 are `untagged` (expected before the code step); FR-161-AC-12 is `tagged` by `tests::relation_backed_entries_are_refused_when_malformed`, FR-161-AC-14 by two tests, FR-165-AC-1 by `tc_1080_011_design_data_is_carried_verbatim`.

What the code step must implement and test: (1) `parse_activities` admits `relation` on `design-trace` and refuses a blank `relation` and any `element_archetypes` (AC-12); (2) `build_rows` routes a pinned `design-trace` through `relation_status` (or equivalent) so only rows in `R` are read (AC-15; unit test with `R` covered/uncovered/absent plus an `uncovered` row in another relation); the AC-14 refusal already holds for any kind through `check_relations_declared`, so AC-15's refusal clause needs only a test; (3) AC-16 and AC-17 tests (oracles to be fixed per SR-2026112016223475846355012); (4) FR-165-AC-20 is a `matrix.build` pass-through test needing no production change. No stub or production code is in the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Once this spec merges, FR-161-AC-12 is tagged by a test asserting the opposite of the amended criterion: `relation_backed_entries_are_refused_when_malformed` asserts `{"kind":"design-trace","relation":"r"}` is refused with "must not", and `parse_activities` refuses it with "design-trace `{id}` must not name a method, relation or element archetypes". The code step must invert that assertion and the refusal message, and add the new blank-`relation` and `element_archetypes` cases; until then the matrix shows AC-12 `tagged` by a contradicting test. | rust/crates/quoin-cli/src/class_obligations.rs:252-257; rust/crates/quoin-cli/src/class_obligations.rs:956-958 |

## Coverage

Plan completion: not assessed

| Criterion | Tests (binding) |
| --- | --- |
| FR-161-AC-12 | relation_backed_entries_are_refused_when_malformed (tag correct; one assertion contradicts the amended criterion; FND-001) |
| FR-161-AC-14 | a_relation_no_loaded_module_declares_is_refused_naming_it; tc_plat1836_007_an_undeclared_relation_is_refused (correct) |
| FR-161-AC-15 | none (untagged, code step) |
| FR-161-AC-16 | none (untagged, code step) |
| FR-161-AC-17 | none (untagged, code step) |
| FR-165-AC-1 | tc_1080_011_design_data_is_carried_verbatim (correct) |
| FR-165-AC-20 | none (untagged, code step) |

## Verdict

No blocking gap for a spec-only PR. The code step's test list is above; FND-001 is a known divergence it must close.
