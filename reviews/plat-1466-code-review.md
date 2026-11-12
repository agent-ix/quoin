---
id: SR-20261112021200
title: "Code review of quoin PR 825 (PLAT-1466 remove Filament IDE references)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@1b6ce909; skills/specify/SKILL.md, skills/spec-to-plan/SKILL.md, .claude/skills/rust-style/SKILL.md, bench/metric-validity.md, rust/crates/quoin-config/src/org.rs, rust/crates/quoin-quire/src/payload.rs, plus the branch-wide `git grep -i filament` completeness sweep"
review_set: subset
---

# Code review of quoin PR 825

## Summary

Ticket: PLAT-1466. Code review with the rust-review lane (repo `.claude/skills/rust-style`
loaded). PR head 1b6ce909 on base c389a557. origin/main is at 0b363d6d (#822), which touches
`spec/matrix.md` only at lines 241 and 1464. Open PR #824 touches it at 145, 240 and 1430.
This PR's only matrix edit is TC-268 at line 1083, so neither one conflicts.

The Rust change is two comments: `quoin-config/src/org.rs:467` and `quoin-quire/src/payload.rs:9`.
Builds ran through `locked-build.sh` with `CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4` and debug=0.
These checks passed:

- `cargo test --locked -p quoin-config -p quoin-quire`, with every suite ok
- `cargo clippy --locked -p quoin-config -p quoin-quire --all-targets --all-features -D warnings`
- `cargo fmt --check`

There is no behaviour change. No test reads the edited skill text. The only skill-text
consumer is `tc_source_conventions.rs`, which checks the rust-style conventions, not the
reworded prose.

Completeness sweep: `git grep -n -i filament` with `reviews/` and `spec/reviews/` excluded. Each
remaining hit falls into one of these classes:

- (a) Review records.
- (b) Recorded data:
  - the quoin-jev `eval-v2`, `ears-m2` and `ears-m6` fixtures;
  - the span-breadth label set, which names a quire-rs document;
  - the criterion-strength fixtures;
  - the agent-eval sentinel baseline screen capture.
- (c) The module-install convention: `IX_FILAMENT_MODULES_PATH`, `~/.ix/filament/{modules,registry.json}`,
  `filamentModulesDir`, the `"Filament module"` / `"Filament catalog"` wording in spec.md,
  default-modules.yaml, FR-007/017/019/024, IT-001/002 and the module crates and their tests.
- (d) Different Filament repos: filament-core-data (ADRs, IT-003, FR-055 and others),
  filament-plan-sync (matrix FR-102 row), the generic "Filament" boundary in FR-054-AC-3 and
  PLAN-006, and the workspace `filament` symlink in TASK-060.

Two hits are (e) IDE leftovers: the "sync into filament-core" sentence in
`skills/spec-review/SKILL.md:97` and `skills/spec-ears-analysis/SKILL.md:90`. No repo named
`agent-ix/filament-core` exists (`gh repo view` fails to resolve it); the repos are
`filament-core-data` and `filament-core-service`. The sentence came in with quoin #9
(f051e99a, "direct-render-then-sync model"). The only code that ingests `SpecReview` artifacts
is `filament-ide-rs`, in `crates/filament-extraction`, the Filament IDE. Neither
filament-core-data nor filament-core-service has a SpecReview ingester. So the sentence is a
stale IDE claim and is false as stated.

No other skill, hook, command, plugin manifest, workflow `def.yaml` or doc tells an agent to
run `filament` or open a `filament-ide://` link. `cockpit` and `deep link` have zero hits.
The renumbered `skills/specify` steps run 1 to 6 with no reference to a step 7.
`skills/spec-to-plan` has no numbered steps that refer to the removed section.

## Verdict

The removals are correct and complete for the skill steps, FR-024/030/043, TC-268, PLAN-003
and the Rust comments, with no behaviour change. Three medium defects remain:

- Two skills still carry the stale filament-core/IDE sync claim (FND-001, FND-002).
- The `bench/metric-validity.md` subject swap leaves the study design inconsistent (FND-003).

There are also two low wording and wrap nits. Not mergeable until FND-001 to FND-003 are fixed.
They are small doc edits that can go in the same PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | IDE leftover in an installed skill. The sentence "The docs sync into filament-core as `SpecReview` artifacts" names a repo that does not exist. The only SpecReview ingester is filament-ide-rs, the Filament IDE, which is now ruled out of scope, so the sentence is false. Delete the sync clause and keep "do not proceed to implementation until reviewed" | skills/spec-review/SKILL.md:97-98 |
| FND-002 | medium | The same stale IDE claim: "The doc syncs into filament-core as a `SpecReview` artifact like every other analysis lens". Delete the sentence | skills/spec-ears-analysis/SKILL.md:90-91 |
| FND-003 | medium | Step 1 now runs `cargo mutants` over "quoin itself", but the rest of the doc still describes the old subject. Line 21 calls the oracle corpus "the tier-2 corpus". Prerequisite 1 requires "the adjudicated tree". Prerequisite 2 says "the tier-2 corpus has not been annotated". Prerequisite 3 cites "24 crates", while quoin has 27. All of these described filament-ide-rs. Swapping the subject silently changes the study design and leaves its prerequisites about a different corpus. Restate the subject and prerequisites together, either as quoin, with its crate count and `implements` coverage, or as a neutral "the tier-2 corpus" without the parenthetical. Line 24 is also re-wrapped past the file's width | bench/metric-validity.md:21-24, bench/metric-validity.md:49-59 |
| FND-004 | low | The `unsafe_code` bullet was joined into one 127-character line. The rest of the file wraps at about 80 characters | .claude/skills/rust-style/SKILL.md:93 |
| FND-005 | low | The comment now reads "a real corpus already exceeded it: a large real corpus (268 spec files, …)", which says "real corpus" twice. Write "a real corpus (268 spec files, 1,107 obligations) emits …" | rust/crates/quoin-quire/src/payload.rs:8-9 |
