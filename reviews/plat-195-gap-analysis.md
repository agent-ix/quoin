---
id: SR-20261103118158
title: gap analysis of the shared semantic-module emit package
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quoin; packages/semantic-module-tools/; spec/functional/FR-076-semantic-module-template-variants.md; rust/crates/quoin-semantic/tests/tc_264_template_render.rs
review_set: subset
date: '2026-11-10'
---

# gap analysis of the shared semantic-module emit package

## Summary

Ticket: PLAT-195 (agent-ix/quoin PR #793, head a97402df). Plan completion: not assessed.

I compared the new requirement FR-076-CON-4 and the moved behaviour against the tests that back them.

## Scope

- `FR-076-CON-4` (examined): backed by tc_264_056 through `#[trace("FR-076-CON-4")]`
- `FR-076-CON-1` (context_only)
- `packages/semantic-module-tools/test/emit.test.mjs` (examined)
- `templates/semantic-module/{{cookiecutter.repo_name}}/tests/test_schema_emission.py` (examined)
- `Makefile`, `pnpm-workspace.yaml`, `.github/workflows/build-test.yml` (examined): checked whether the package tests run anywhere

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Nothing in quoin runs the package's `node --test` suite. The pnpm workspace is `.` only, the Makefile has no node test target, and build-test.yml never calls it. The four tests that back the moved emit logic never run in the gate. | packages/semantic-module-tools/package.json:20-22 |
| FND-002 | low | No package test covers `checkOutput` (changed and stale-file detection), `writeOutput` (deleting a stale .json), the explicit `--semantic-core-version` path, or CLI exit codes. They are exercised only by a rendered repository's pytest, which quoin CI does not run. | packages/semantic-module-tools/src/emit.mjs:203-235 |

## Details

- **FND-001**: Wire the suite into `make test` (or `rust-gate`) so CI runs it. It needs no dependencies, only `node --test packages/semantic-module-tools`.
- **FND-002**: These are small, pure filesystem tests over a temp directory, and they lock in the drift gate's behaviour within quoin itself.

## Verdict

FR-076-CON-4 is backed: tc_264_056 asserts the exact-version dev dependency, the absent local driver, the absence of any `tsp compile` in `scripts/*.mjs`, and that the poe task calls the bin, across all three variants. I confirmed each assertion by rendering all three variants by hand. All four package tests pass locally, and the rendered `test_schema_emission.py` passes (7 tests).

The ticket asks for the package behaviour to be recorded in an emitted `toolchain.json`. Per the coder, no such file exists on origin/main, and recording it would be version tracking, so that criterion is stale rather than a gap.

Mergeable with respect to this method once FND-001 is fixed.
