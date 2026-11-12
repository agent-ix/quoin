---
id: SR-20261112013804
title: "Spec review of quoin PR 819 (NFR-007 rewritten to ix-flow only)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@7ee1809f; spec/non-functional/NFR-007, NFR-001, NFR-004, NFR-012, spec/non-functional/index.md, spec/functional/FR-015, FR-021, FR-036, spec/assurance/MP-252, spec/matrix.md, spec/spec.md"
review_set: subset
---

# Spec review of quoin PR 819

## Summary

Ticket: PLAT-1452. Spec-review with the integrity (cross-reference consistency) and EARS
sub-checks, scoped to the NFR-007 rewrite and to every artifact that references NFR-007.
`quire validate` (0.40.0) on the changed files: NFR-007 passes with no diagnostics. Three
diagnostics are not touched by this diff: the MP-252 `population_set` frontmatter error, the
matrix TC-1943/1944 tombstone rows, and the NFR-001 `ears:non-singular` warning. They are
not attributed to it.

Checked true against the code:

- "invoked by name from PATH": `runner_program()` returns `ix-flow` (`ix-flow.cmd` on Windows)
  as a bare name (flow.rs:293-299).
- "absence or non-zero exit surfaces as non-zero": `execute` maps a spawn failure to
  `could not start ix-flow` and maps a failing status to an error. TC-3004 runs each workflow
  command with ix-flow absent and failing.
- "quire is a linked library": the quoin-quire Cargo.toml has `quire-rs =0.56.0`.
- "the only processes are git, ix-flow, $EDITOR": FR-036-AC-8 / TC-203,
  `EXPECTED_EXECUTABLES`.
- "not version-pinned": flow.rs requires ix-flow >= 0.3.1. That is a floor, not a pin, so
  the claim holds.
- Dropping FR-015 and FR-022 is correct. NFR-007 no longer constrains the emitted command or
  self-update, which fetches over HTTP. FR-021 is kept.

## Verdict

The NFR-007 rewrite itself is true and adds no untested claim. The two medium findings are
in artifacts that still describe the old NFR-007. Fix them in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | NFR-004 still says `ix-flow` and `quire` SHALL be invoked as external processes rather than linked, with metric "External tools linked rather than invoked as processes = 0". The matrix NFR-004 row says "`ix-flow`/`quire` are spawned, not imported". Both now contradict the rewritten NFR-007 Statement ("`quire` is a library linked into `quoin`, not a process") and the code (quoin-quire links quire-rs). This is pre-existing TS-era staleness, and this PR makes it a direct contradiction between two NFRs | spec/non-functional/NFR-004-standalone-dependencies.md:17, spec/non-functional/NFR-004-standalone-dependencies.md:36, spec/matrix.md:149 |
| FND-002 | medium | FR-015 Dependencies still says "`quoin` emits it rather than executing it (see NFR-007)". NFR-007 no longer carries that obligation, so the pointer dangles. The claim is also no longer generally true: the workflow provider runs quire validation in process (flow/provider.rs:479) | spec/functional/FR-015-emit-quire-validate-command.md:50-52 |
| FND-003 | low | The non-functional index still carries NFR-007's old title, "External tools are invoked by name and surface their failures". spec.md was updated but this index was not | spec/non-functional/index.md:17 |
| FND-004 | low | NFR-012 (quire/module version skew) keeps a `constrains` edge to NFR-007 and describes it as "(external tools resolved from `PATH`)". quire is no longer resolved from PATH, and NFR-007 now covers ix-flow only, so the edge and gloss are stale | spec/non-functional/NFR-012-ecosystem-compatibility.md:11, spec/non-functional/NFR-012-ecosystem-compatibility.md:68 |
| FND-005 | low | NFR-007 Verification cites "`tc_203` (FR-036-AC-8)" as if it were a test symbol. No such function exists. The test is quoin-core `tests/tc_source_conventions.rs` :: `quoin_executes_exactly_git_and_ix_flow_and_the_operators_editor` (TC-203) | spec/non-functional/NFR-007-external-tool-invocation.md:44 |
