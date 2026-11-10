---
id: SR-20261103118170
title: "Gap analysis of PLAT-212 import_types implementation (quoin#797)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@f1e6b687; FR-070-AC-8..13, FR-075-AC-6..7, FR-078-AC-9, FR-124-AC-9; spec/matrix.md TC-2130..2139; rust/crates/quoin-semantic, rust/crates/quoin-cli/tests"
review_set: subset
---

# Gap analysis of PLAT-212 implementation (quoin#797)

## Summary

Planless gap analysis over the PR diff. Plan completion: not assessed. Ticket: PLAT-212.
Each criterion was matched to the test carrying its `#[trace("AC", "TC")]` marker, and the
test body was read to check it asserts the criterion. Matrix rows TC-2130..2139 move to
Complete and every one has a same-marker test. No stubs, no `todo!()`, no production code
without an owning requirement.

Bindings: FR-070-AC-8/TC-2130 tc_212_001; AC-9/TC-2131 tc_212_002; AC-10/TC-2132 tc_212_003;
AC-11/TC-2133 tc_212_004; AC-12/TC-2134 tc_212_005 (+ CLI tc_212_002 for install success);
AC-13/TC-2139 tc_212_006; FR-075-AC-6/TC-2136 tc_212_007; FR-075-AC-7/TC-2137 tc_212_008;
FR-078-AC-9/TC-2138 tc_264_020; FR-124-AC-9/TC-2135 CLI tc_212_001 and tc_212_002. All correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-070-AC-13 states the locus as `<section>[T]`; only `object_types[...]` is tested. The locus fix for `artifact_types` (previously reported under `object_types[...]`) has no test, so a regression to the hard-coded section would pass | rust/crates/quoin-semantic/tests/tc_212_import_types.rs:339-370; rust/crates/quoin-semantic/src/manifest.rs:483 |
| FND-002 | medium | No test asserts that a `$ref` to an undeclared package still yields `semantic.schema-ref-unshipped` after the walker change; tc_212_005's `elsewhere` case only checks `resolve_imports` output, not the read diagnostics, so dropping the final `else` refusal would pass | rust/crates/quoin-semantic/tests/tc_212_import_types.rs:312-325; rust/crates/quoin-semantic/src/data_schema.rs:528-532 |

## Verdict

**PASS WITH FINDINGS (two medium).** Every new criterion is backed by a test that asserts
it, with the right TC id on the same marker; matrix status changes are honest. Two
behaviours the change relies on are untested: the `artifact_types` locus and the continued
refusal of undeclared-package refs. Add one case each (an `artifact_types` entry with an
unexported `$ref`, asserting `artifact_types[<T>].data_schema.schema`; and the `elsewhere`
case asserting `schema-ref-unshipped` in `read(...).diagnostics`).

## Scope

- `FR-070-AC-8` (examined): manifest with imports and no import_types loads as before; with import_types the pack reports the lists.
- `FR-070-AC-9` (examined): six malformed shapes plus non-object rejected naming package and entry.
- `FR-070-AC-10` (examined): unexported import_types entry fails install with semantic.unknown-import-type naming package and type.
- `FR-070-AC-11` (examined): provider not installed raises only the unresolved-import error.
- `FR-070-AC-12` (examined): unreferenced import yields semantic.unused-import warning, install succeeds; referenced import yields none.
- `FR-070-AC-13` (examined): $ref naming an unexported type fails at `<section>[T].data_schema.schema`, with and without a list; an exported type raises none.
- `FR-075-AC-6` (examined): derived import entry exports equals the declared list, [] when none.
- `FR-075-AC-7` (examined): cycle naming over type nodes only when type edges cycle; packages otherwise; always fails.
- `FR-078-AC-9` (examined): rendered semantic block has no import_types key.
- `FR-124-AC-9` (examined): inspect reports unknown-import-type for list and $ref (exit 1) and unused-import warning (exit 0 alone).
