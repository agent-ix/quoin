---
id: SR-20261112191018
title: "Gap analysis of quoin PR 831 against FR-137, FR-138 and FR-139"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@5cd35f5c; spec/functional/FR-137-detect-and-place-licensed-packs.md, spec/functional/FR-138-packs-never-come-from-hosted-sources.md, spec/functional/FR-139-pack-writes-nothing-into-a-repository.md, and the PR's tests and production diff"
review_set: subset
---

# Gap analysis of quoin PR 831

## Summary

Ticket: PLAT-1492 (and PLAT-1493). Planless. Plan completion: not assessed. Every acceptance
criterion in FR-137 (AC-1..6 and AC-8..12; the AC-7 gap is the lead's accepted drop),
FR-138 (AC-1..5) and FR-139 (AC-1..6) has at least one `#[trace]`-tagged test in the PR, and
all of those tests pass. The traces were read from the test sources, and `quoin matrix` was not
run. FR-137-AC-12's inspection half holds: no `TextRights`, `Licensed` or `clearance_ref` appear
in non-test source. The witness is a fn-pointer const, so removing `check_publishable` fails the
build.

## Verdict

The behaviour is implemented and traced. Two criteria are only partly covered by their tagged
tests. Test-strength gaps in AC-4, AC-6, AC-9 and AC-10 are in the code-review artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-137-AC-3 names sets that are `text: none`, `original` or `explicitly_cleared`, but tc_1492_002 and tc_1492_102 cover only `none` and `original`. Add an `explicitly_cleared` set (with its clearance reference) to one of them | rust/crates/quoin-quire/src/clauses.rs:917-923; rust/crates/quoin-cli/tests/tc_1492_licensed_packs.rs:84-104 |
| FND-002 | low | FR-138-AC-1 lists a package source. tc_1492_040 covers git, git-subdir and github only. `Source::Npm` fails earlier as `UnsupportedSource`, so a package pack is refused without the module, source kind or sets that FR-138 requires. Either test package and accept the unsupported refusal by stating it in FR-138, or drop "package" from AC-1 while npm sources are unsupported | rust/crates/quoin-modules/tests/tc_1492_packs.rs:150-168; rust/crates/quoin-modules/src/install.rs:270 |
