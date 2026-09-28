---
id: SR-171
title: "Code review — quoin#649 module-pin bump (spec-artifacts-process v0.27.0, spec-artifacts-iso v0.20.0)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@bac91efc1a06266ad9cf513c2bcc0ea0d65f0410; default-modules.yaml, rust/crates/quoin-cli/tests/fixtures/retained-catalog/ix-home/filament/registry.json, rust/crates/quoin-modules/tests/tc_1032_pin_agreement.rs, rust/goldens/default-modules.yaml, rust/goldens/PROVENANCE.md, quality/verification-stack-lock.json"
review_set: subset
---

## Summary

Ticket: PLAT-1079 (and PLAT-1085). Reviewed the two-file config diff of quoin#649 at `bac91ef`. Both new refs resolve to annotated tags whose peeled commits are the merge commits of spec-artifacts-process#106 (`2ec1d3e`) and spec-artifacts-iso#46 (`4637b44`). Every live pin copy moved; the two deliberately unchanged files are correctly left alone.

## Verdict

**PASS** — the diff is correct and complete for what it changes. No duplication was introduced: the fixture registry is a pre-existing in-repo mirror that `tc_1032` already guards. No Rust sources changed, so the rust-review lane does not apply. No AssuranceProfile scopes module pins. AP-201 and AP-202 cover finding quality and Jev evaluation.

Checks performed:

- `git ls-remote` gives `refs/tags/v0.27.0^{}` = `2ec1d3e29c29…` and `refs/tags/v0.20.0^{}` = `4637b441909e…`. Both equal the `mergeCommit` of the named PRs (`gh pr view`), and both PRs are MERGED.
- A grep for `v0.26.0|v0.27.0|v0.19.0|v0.20.0` and the matching version strings, outside `target/`, finds the new refs only in `default-modules.yaml` and the retained-catalog fixture. No live copy was missed. The other matches are prose in `spec/log.md` and `reviews/`, plus unrelated crate versions.
- `tc_1032_every_engineering_assurance_pin_is_the_same_commit` walks **every** `default-modules.yaml` entry and asserts that the fixture registry's `ref` is equal to it. That covers both bumped modules.
- `rust/goldens/default-modules.yaml` is a frozen TS-oracle capture. It pins process at `d605caa` and iso at `v0.18.0`, so it was already behind the old pins. `PROVENANCE.md` says "Nothing here asserts a particular upstream pin", and precedent 3f9621d did the same. Leaving it is correct.
- `quality/verification-stack-lock.json` pins process at `61a20e0` and iso at `a6b1c70`, which is not the old v0.26.0/v0.19.0 either. Nothing in `rust/`, `scripts/`, `Makefile` or `.github/` reads it. It is an independently relocked campaign, and leaving it is correct.
- `cargo test -p quoin-modules` at `bac91ef` passes: 60 tests across 7 binaries, including both `tc_1032` tests.
- Impact measurement: `quire validate --summary 'spec/**/*.md'` over quoin's own spec, with the other eight installed modules, was run once with process v0.26.0 / iso v0.19.0 and once with v0.27.0 / v0.20.0. Both runs report the same 26 pre-existing failing documents, and the bump adds no diagnostic.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low      | No findings (placeholder) | - |

## Dispositions

Round 1, reviewed at bac91ef: there is nothing to dispose. FND-001 is the `(placeholder)` row and records no defect, so no outcome row applies.
