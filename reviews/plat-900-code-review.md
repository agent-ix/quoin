---
id: SR-20261103118172
title: "Code review of quoin PR 801 (FR-070 cleanup, registry-keys test)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@fdfb889; rust/crates/quoin-semantic/tests/tc_452_contract_criteria.rs, rust/crates/quoin-semantic/DIVERGENCE.md, spec/reviews/module-cookiecutter-dependency.md"
review_set: subset
---

# Code review of quoin PR 801

## Summary

Ticket: PLAT-900. Scoped to `git diff origin/main...HEAD` at fdfb889. Rust lane
(`rust-review`) folded in. `cargo test -p quoin-semantic --test tc_452_contract_criteria`
run through locked-build.sh: 4 passed, 0 failed.

The new test `tc_452_644` compiles the pinned module-manifest schema, validates a manifest
that carries all eight registry keys with well-formed values, and asserts no errors. It then
adds `not_a_registry` and asserts the result is invalid. The schema root has
`additionalProperties: false`, so the test fails if any of the eight keys is no longer
declared, if a value shape stops matching, or if the root starts admitting unknown keys. The
negative half cannot pass vacuously, because the same manifest was valid one line earlier.
Idiom is consistent with the file: `expect` in a test under the file's existing lint allow,
`json!` imported once, and the trace attribute names AC-14 and TC-2140 like the sibling tests.
Retagging `tc_452_640` to FR-070-AC-3 is a sound binding: the test asserts that the schema's
`semantic` keys equal the contract's admitted list, which is the acceptance half of AC-3.
The DIVERGENCE.md ownership correction is accurate: all three schemas ship in
filament-core-data's `@agent-ix/semantic-schema`.

## Verdict

**PASS WITH FINDINGS (one medium).** The Rust is clean. The PR rewrites a historical review
record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR rewrites a committed review record. It changes the EXT-MANIFEST evidence row and the text of finding FND-006 in spec/reviews/module-cookiecutter-dependency.md, deleting FND-006's clause about the vendored copy's CON-2 provenance fields. A review records what was observed when it ran. Policy is never to delete or rewrite reviews. Revert both hunks to their origin/main text | spec/reviews/module-cookiecutter-dependency.md:49, spec/reviews/module-cookiecutter-dependency.md:129 |

## Examined scope

```yaml
clean: false
scope:
- {id: tc_452_644, path: rust/crates/quoin-semantic/tests/tc_452_contract_criteria.rs, role: examined, excerpt: "fn tc_452_644_a_manifest_with_all_eight_registry_keys_validates()"}
- {id: tc_452_640, path: rust/crates/quoin-semantic/tests/tc_452_contract_criteria.rs, role: examined, excerpt: "#[trace(\"FR-070-AC-3\")] fn tc_452_640_the_module_manifest_schema_admits_the_contract_keys()"}
- {id: tc_452_641, path: rust/crates/quoin-semantic/tests/tc_452_contract_criteria.rs, role: examined, excerpt: "fn tc_452_641_the_pinned_schema_adds_no_required_key_versus_pre_cr003() (rename and doc only)"}
- {id: DIVERGENCE.md, path: rust/crates/quoin-semantic/DIVERGENCE.md, role: examined, excerpt: "module-manifest.schema.json, package-manifest.schema.json and common.schema.json are owned by filament-core-data"}
- {id: module-cookiecutter-dependency, path: spec/reviews/module-cookiecutter-dependency.md, role: examined, excerpt: "EXT-MANIFEST row and FND-006 text rewritten"}
```
