---
id: SR-20261112020055
title: "Spec review of quoin PR 824 (PLAT-1464 FR-131, NFR-003, NFR-007, MP-250..252, matrix)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@4faa1137; spec/functional/FR-131-*.md, spec/non-functional/NFR-003-*.md, spec/non-functional/NFR-007-*.md, spec/assurance/MP-250..252-*.md, spec/matrix.md"
review_set: subset
---

# Spec review of quoin PR 824

## Summary

Ticket: PLAT-1464. Scoped to the spec and plan edits in `git diff origin/main...HEAD`. The
integrity and scope checks are folded in.

- **FR-131-AC-9 and AC-10.** The rewording is true to the code. There are seven classes,
  each enumerated from `--help`; the declared set equals the enumeration (`verdict()`
  assert); intake admits the observation; `verify` reads it. AC-10 names the twelve modes the
  stub stages.
- **TC ids.** TC-3700..3706 are unused on origin/main. Open PR #822 uses TC-3600..3606, so
  the ranges do not overlap. `quoin matrix` reports no criterion tagged outside its row.
- **Validation.** `quire validate` on the changed files reports `population_set` unexpected
  in MP-250/251/252. That error is pre-existing: MP-253 on main gets the same error, because
  the installed spec-artifacts-process v0.31.0 schema lags. It is not this PR's.
- **Merge state.** A test merge with origin/main (c389a557, #819) conflicts in
  `spec/matrix.md` (NFR-003..007 rows) and in the NFR-007 Verification section. PR #822
  edits the FR-132 summary row, which sits next to this PR's FR-131 row, so whichever of
  the two lands second rebases.

## Verdict

The spec edits are accurate apart from the wording nits below. The PR is not mergeable until
it is rebased on main and the conflicts are resolved as FND-001 and FND-002 describe.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Conflicts with origin/main (#819) in spec/matrix.md and the NFR-007 Verification section. Resolve as follows. Keep main's NFR-004 row. Keep main's TC-203 paragraph in NFR-007 Verification. Take the PR's M-1 paragraph, but drop "NFR-007-M-2 has no producer." (#819 removed M-2). Drop the M-2 sentence from the NFR-007 matrix row | spec/matrix.md:148-152, spec/non-functional/NFR-007-external-tool-invocation.md:48-60 |
| FND-002 | medium | After the rebase, the NFR-007 row should be ✅ Covered, not 🚧 Partly. Its only metric, M-1, reads 24/24 at target, and TC-3004 asserts every pair matched plus an Accept. Name-from-PATH is shown by the absent mode and TC-203. AC-1 is a Demonstration of an accepted limitation | spec/matrix.md:152 |
| FND-003 | low | The NFR-003 matrix row says the below-target pairs "omit the path or the available types, or exit zero without a diagnostic". That misses write-record, which never says the input is JSON or names --input. It also misses validate's blank path. And completeness exits 0 with an unrelated stderr hint, not with no diagnostic | spec/matrix.md:148 |
| FND-004 | low | MP-250 says commands that read their help differently from how they behave, "such as write's directory", are "probed as they behave". No producer code does anything of the kind. write's `[REPO_DIR]` is bracketed and is probed as an ordinary positional target. Remove or correct the sentence | spec/assurance/MP-250-nfr003-actionable-failures.md |
| FND-005 | low | MP-250 Interpretation says the below-target "set can only shrink". The ratchet fails in both directions: fixing a command fails its TC-37xx test until the id is removed from the list. Say a fix must remove the id | spec/assurance/MP-250-nfr003-actionable-failures.md |
