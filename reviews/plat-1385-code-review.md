---
id: SR-17076
title: "Code review of quoin PR 798 (remove the semantic-module template and tools package)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@9a6555ac; .github/workflows/build-test.yml, .github/workflows/release-semantic-module-tools.yml, .gitignore, Makefile, pnpm-workspace.yaml, packages/semantic-module-tools/**, templates/semantic-module/**, rust/crates/quoin-semantic/tests/{common/mod.rs,common/template.rs,tc_194_template_refusals.rs,tc_264_template_render.rs}, rust/crates/quoin-semantic/Cargo.toml"
review_set: subset
---

# Code review of quoin PR 798

## Summary

Ticket: PLAT-1385. Scoped to `git diff origin/main...HEAD` at 9a6555ac (one commit over
origin/main a7d60bae). Includes the Rust lane (`rust-review`). Read-only: no gate rerun; the
coder reported `make test` green on this head.

Checked: every deleted path (`templates/semantic-module/**`, `packages/semantic-module-tools/**`,
`release-semantic-module-tools.yml`, `common/template.rs`, `tc_194_*`, `tc_264_*`) has no
surviving reference in Makefile, workflows, pnpm-workspace, .gitignore, README/AGENTS/CLAUDE/docs,
or Rust sources. `make test` now depends only on `rust-gate`; the CI job that ran `make package-test`
is gone and no other job `needs:` it. `tc_90_module_sizes` has an empty allow-list, so no size entry
names a deleted file. `pub mod template;` is removed from `tests/common/mod.rs` and no other test
uses `common::template`.

## Verdict

**APPROVE with one low finding.** Deletions are complete and consistent with the owner direction
(replaced by the external cookiecutter repo, no shim). The only residue is a dev-dependency whose
sole consumers were the deleted tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `regex` stays a dev-dependency of quoin-semantic, but its only users were the deleted `tc_194_template_refusals.rs` and `tc_264_template_render.rs`; no remaining quoin-semantic source or test imports `regex` | rust/crates/quoin-semantic/Cargo.toml:60 |
