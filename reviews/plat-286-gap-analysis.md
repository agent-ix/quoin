---
id: SR-17074
title: "Gap analysis of quoin PR 783 (FR-124 quoin module inspect)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@971b6885; spec/functional/FR-124-inspect-installed-semantic-modules.md against rust/crates/quoin-cli/tests/tc_module_command.rs, rust/crates/quoin-cli/src/module.rs and rust/crates/quoin-core/src/ops/semantic"
review_set: subset
---

# Gap analysis of quoin PR 783

## Summary

Ticket: PLAT-286. Plan completion: not assessed. Planless audit of FR-124's five acceptance
criteria and its Behavior clauses against the four new tests and the code. `quoin matrix --repo
.` shows FR-124-AC-1 to AC-5 all `tagged` (no run evidence bound, which is normal before
`make evidence`). Tools: quoin 0.29.1, quire 0.36.2 (engine 0.50.2).

Bindings checked: tc_286_001 asserts AC-1 and AC-2, tc_286_002 asserts AC-3, tc_286_003 asserts
AC-4, and tc_286_004 asserts AC-5. Every binding is correct. Each test fails against a stub: they
assert literal modes, versions, exit codes and message fragments from a real installed set, so
none is vacuous. No code without an owning requirement was found. The `read_blocks` wire
additions are owned by FR-124 and FR-075.

## Verdict

**PASS WITH FINDINGS (two low).** Every criterion has a real test. Two tests assert less than
their criterion states, and two Behavior clauses have no criterion and no test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | tc_286_001 asserts only half of what it traces. AC-1 says each module is reported "with its package and contract version", but the compiled entry's `package` and `contract_version` are not asserted. AC-2 says "every entry's error diagnostics are empty", but only the compiled entry's errors are checked, not the dynamic entry's | rust/crates/quoin-cli/tests/tc_module_command.rs:209-238 |
| FND-002 | low | Two FR-124 Behavior clauses have no criterion and no test: "one error per import cycle", and "inspect SHALL leave the registry, every installed module and every consuming repository unmodified". The unreadable-module case (code-review FND-001) is also untested, which is how that defect went unnoticed | spec/functional/FR-124-inspect-installed-semantic-modules.md:51,58 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-124, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "An entry's `diagnostics` SHALL be the diagnostics of reading its block (FR-070, FR-073, FR-074) followed by one error per `semantic.imports` entry that no other installed module provides at exactly the locked version, naming the import, the locked version and the installed versions, and one error per import cycle. `inspect` SHALL leave the registry, every installed module and every consuming repository unmodified."}
- {id: FR-124-AC-1, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "With a dynamic-only module and a module declaring a `rust` target installed together, `inspect` reports `dynamic` for the first and `compiled` for the second, each with its package and contract version."}
- {id: FR-124-AC-2, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "A clean installed set exits zero and every entry's error diagnostics are empty."}
- {id: FR-124-AC-3, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "A module with no `semantic` block is listed as `no-semantic-block` and the command exits zero."}
- {id: FR-124-AC-4, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "When an imported module is reinstalled at a version other than the locked one, `inspect` exits 1, writes the whole document, and reports on the importer an error naming the import, the locked version and the installed version."}
- {id: FR-124-AC-5, path: spec/functional/FR-124-inspect-installed-semantic-modules.md, role: examined, excerpt: "A block whose `contract_version` is `2.0.0` is reported with an error diagnostic naming `2.0.0` and no `semantic` view, and the command exits 1."}
bindings:
- {test_id: tc_286_001_inspect_reports_dynamic_and_compiled_modules_together, ac_id: FR-124-AC-1, trace: correct}
- {test_id: tc_286_001_inspect_reports_dynamic_and_compiled_modules_together, ac_id: FR-124-AC-2, trace: correct}
- {test_id: tc_286_002_a_module_without_a_semantic_block_is_reported_and_clean, ac_id: FR-124-AC-3, trace: correct}
- {test_id: tc_286_003_an_import_lock_the_installed_set_no_longer_satisfies_is_reported, ac_id: FR-124-AC-4, trace: correct}
- {test_id: tc_286_004_an_unknown_contract_major_is_reported_not_degraded_to_empty, ac_id: FR-124-AC-5, trace: correct}
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | tc_286_001 now asserts the compiled entry's `contract_version` 1.0.0 and `package` agent-ix/spec-objects-consumer, and `errors(dynamic).is_empty()` |
| FND-002 | fixed | FR-124-AC-7 (cycle) and AC-8 (read-only) were added with tc_286_006, which edits an installed manifest into a two-module cycle and asserts an "import cycle:" error naming both packages, and tc_286_007, a byte snapshot of `filament/` before and after an erroring run. The unreadable case is AC-6 / tc_286_005 and tc_452_602. All pass, and `quoin matrix` shows AC-6 to AC-8 tagged |
