---
id: SR-20261113152318
title: "Code review (Rust lane) — PLAT-1536 quoin profile evaluate and source-profile loader (quoin PR 875)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/src/profile/mod.rs, rust/crates/quoin-cli/src/profile/model.rs, rust/crates/quoin-cli/src/profile/evaluate.rs, rust/crates/quoin-cli/src/profile/model/tests.rs, rust/crates/quoin-cli/src/profile/evaluate/tests.rs, rust/crates/quoin-cli/tests/tc_1536_profile_evaluate.rs, rust/crates/quoin-cli/src/help.rs, rust/crates/quoin-cli/src/main.rs, rust/crates/quoin-cli/src/tests.rs, rust/crates/quoin-quire/src/clauses.rs, rust/crates/quoin-cli/Cargo.toml, rust/Cargo.lock"
review_set: subset
---

## Summary

Ticket: PLAT-1536. PR head `a550f49a`, diff `git diff origin/main...HEAD` (12 files, +1914). The
Rust lane (quoin-local rust-style, then dev-tools rust-style) is folded in. Checks run at this head,
in a scratch worktree with its own target dir, through `locked-build.sh`, `-j 4`, debug=0:

- `cargo fmt --check`: clean.
- `cargo clippy -p quoin-cli -p quoin-quire --all-targets --all-features -- -D warnings`: clean.
- `cargo test -p quoin-cli --no-fail-fast`: `tc_1536_profile_evaluate` 21 passed, and the unit
  tests pass. Two suites this PR breaks fail: `tc_retained_help_capture` (2) and
  `tc_3003_nfr_populations` (1). Both pass on origin/main `ff8bc422` in the same environment.
- `tc_145_risk_advise_command` (1 fail) and `tc_1515_soup_promote` (15 fail) are caused by the
  environment, not by this PR. They fail the same way on origin/main when
  `SPEC_OBJECTS_SAFETY_DIR` / `SPEC_OBJECTS_SECURITY_DIR` point at the stale sibling checkouts
  `/home/peter/dev/spec-objects-{safety,security}` (2026-09-21, branch `bump-semantic-core-0.3.0`).
  They pass on both the PR head and main with fresh origin/main checkouts of those repos.
  `tc_154`, `tc_169` and `tc_136` behave the same way.

The built binary was run against a synthetic module to reproduce FND-003 and FND-004.

## Verdict

The design is sound. The loader, merge, evaluation, summary and closed report types match FR-180,
FR-181 and FR-182. The engine is used through a narrow `Engine` trait, so the unit tests can supply
a failing or malformed engine. There is no `unsafe`, no panic path in production code, and every
test module's `#[allow]` carries the repo's standard reason. No licensed pack is named and no
standard text appears: the fixtures are `example.invalid`, with a sentinel the test asserts never
reaches output. No build artifact, roadmap content or vendored file is in the diff.

The PR is **not mergeable**:

- It leaves two gate tests red (FND-001, FND-002).
- A repository's selection file can read a profile from outside every module `profiles/`
  directory (FND-003).
- The new repo-controlled YAML input has no resource bound (FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `tc_retained_help_capture` fails at this head: `tc_1650_native_root_help_matches_the_retained_capture` and `tc_1650_native_help_preserves_retained_routes_and_current_workflow_paths`. `help.rs` adds a `profile` row to the root COMMANDS list, but the root `--help` stdout in `tests/fixtures/retained-command-help.json` was not updated. Every earlier command PR (#865, #867, #866, #852) updated that fixture in the same change. It passes on origin/main. Fix: add the `profile  Evaluate the selected source profile.` line to the fixture's root `--help` capture. | rust/crates/quoin-cli/src/help.rs:96; rust/crates/quoin-cli/tests/fixtures/retained-command-help.json |
| FND-002 | high | `tc_3003_the_nfr003_populations_are_stated_by_class` fails at this head: "MP-250: the declared set differs from the one the code enumerates". The code now enumerates `profile:unknown-flag`, but MP-250's `population_set` does not list it. It passes on origin/main. Fix: add `profile:unknown-flag`, the only member the code enumerates beyond the declared set, to `spec/assurance/MP-250-nfr003-actionable-failures.md`. | spec/assurance/MP-250-nfr003-actionable-failures.md:16; rust/crates/quoin-cli/tests/nfr_populations/mod.rs:156 |
| FND-003 | high | The profile name from the repository's selection file reaches a path unchecked: `module.root.join("profiles").join(format!("{name}.yaml"))`. An absolute name replaces the module root, and `..` walks out of it. `parse_profile` is then given the selection string as the "stem", not the real file stem, so the `name == stem` rule does not catch it. Reproduced: `.ix/source-profile.yaml` with `profile: <repo>/evil`, plus `<repo>/evil.yaml` whose `name` is the same string, evaluates with exit 0. This breaks FR-180's "read the selected profile from the module `profiles/` directories and from nowhere else" and defeats FR-180-AC-10. Fix: refuse a profile name that is not a single plain stem (no separator, no `..`, not absolute), ideally the FR-020 name grammar. Pass `file.file_stem()` to `parse_profile`. Add a test for both forms. | rust/crates/quoin-cli/src/profile/model.rs:185-214 |
| FND-004 | medium | The repo-controlled `.ix/source-profile.yaml` (and the user file and profile files) is read with an unbounded `read_to_string` and parsed by `quoin_yaml`, which resolves aliases by expansion. Reproduced: a 361-byte alias-chain selection file took 8.6 s and 2.6 GB RSS before being refused for an undefined key. One more level exhausts memory. That breaks the repo's untrusted-input rule that every accumulator has a ceiling and a named refusal. Fix: read through a bounded reader (as `quoin-store` `bounded_read` / flow `read_bounded` do) with a stated byte ceiling, and refuse aliases in these small config files (or cap expanded nodes). The alias expansion itself lives in quoin-yaml and affects other readers too. | rust/crates/quoin-cli/src/profile/mod.rs:91-97; rust/crates/quoin-cli/src/profile/model.rs:209; rust/crates/quoin-yaml/src/lib.rs |
| FND-005 | medium | Every refusal goes out as `Err(String)`, which is exit 1 ("command failure") with prose only. That covers a missing profile, a duplicate, a bad profile or selection, an ambiguous key, a pack in the project layer, an engine failure and an invalid report. In the workspace taxonomy exit 1 is `Partial`, and the newer native commands (`soup`, `risk`) refuse with `Outcome::Refused` (2) and a stable diagnostic code. A caller cannot tell these conditions apart, or tell a refusal from a partial result, without parsing message text. rust-style: "An operation picks a CODE, never an exit number"; "each condition a caller must distinguish gets its own code". Fix: return a refused `Response` with one diagnostic code per `ProfileError` / `EvaluateError` variant. | rust/crates/quoin-cli/src/profile/mod.rs:48-55,115-152 |
| FND-006 | low | The doc comment on `write_reports` says files are "named by position so no key text reaches a path". The code puts the sanitised key `id` into the name (`{:02}-{safe}.json`). The sanitisation (ASCII alnum, `-`, `.`, else `_`, behind a numeric prefix) is safe against traversal. The comment is false. Fix: correct the comment, or drop the id if no key text is intended in the file names. | rust/crates/quoin-cli/src/profile/mod.rs:155-174 |
| FND-007 | low | On a duplicate, `load_profile` discards the module it already has in `found` (`Some((_, file))`). It then re-derives the first module by `file.starts_with(&m.root)`, with a `String::new` fallback, so a root the prefix test misses gives "shipped by both `` and …". Fix: keep the first `(module, file)` pair and use its `name`. | rust/crates/quoin-cli/src/profile/model.rs:193-208 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| `profile/model.rs` parse_profile, parse_selection, parse_user_add, effective | examined | Rules match FR-180/FR-182; FND-003, FND-007 |
| `profile/model.rs` load_profile | examined | FND-003, FND-007 |
| `profile/evaluate.rs` evaluate, validate, summary, closed `Evaluation` type | examined | Correct; closed via `deny_unknown_fields` and an enum schemaVersion |
| `profile/mod.rs` command, run, module_roots (catalog.load), user_root, profile_evaluate, write_reports, QuireEngine | examined | FND-004, FND-005, FND-006 |
| `quoin-quire` `declared_clause_sets` | examined | Correct; strict load only when the manifest lists clause sets |
| `help.rs`, `main.rs`, `tests.rs` route census | examined | FND-001, FND-002 |
| unit and integration tests | examined | They assert real outputs through the binary; coverage gaps are in the gap-analysis SR |
| `quoin-core` `catalog.load` (first-wins module names, semantic read) | context_only | Read to judge the coder's module-root note |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The new integration test `an_alias_bomb_or_an_oversized_selection_file_is_refused_cheaply` has no `#[trace(...)]` marker. quoin rust-style says test functions carry a `#[trace]` naming the criteria they assert. It asserts FR-180's refusal of a malformed selection file (FR-180-AC-6's class) and the new bound. It runs and passes, but the computed matrix cannot see it. Fix: tag it, for example `#[trace("FR-180-AC-6")]`. This does not block merge. | rust/crates/quoin-cli/tests/tc_1536_profile_evaluate.rs:441-471 |

## Dispositions

Round 1 is checked against fix commits `c0079da7` and `5244020b` (head `5244020b`, rebased on
`ff8bc422`). Gates were run at this head with fresh origin/main sibling checkouts
(spec-objects-safety `bf44b49`, spec-objects-security `ee4514b`; `SPEC_OBJECTS_SECURITY_DIR`
pointed at its `spec_objects_security` subdirectory):

- `cargo test -p quoin-cli -p quoin-yaml`: all 39 suites green. tc_retained_help_capture 6/6,
  tc_3003 13 passed + 1 ignored, tc_1536 24/24, tc_145 4/4, tc_1515 16/16, tc_154, tc_169, tc_136,
  tc_162.
- clippy `-D warnings` (quoin-cli, quoin-yaml, quoin-quire, all targets and features), fmt and
  `cargo deny check` are clean.

`saphyr-parser =0.0.12` is already in the graph as saphyr's own dependency, a single lockfile
entry, so it adds no new crate or licence.

I probed the alias scanner with the built binary. It refuses aliases in a merge key (`<<: *b`), in
flow style, as a mapping key, in a later document, and in the profile and user files, all with the
"YAML aliases are not accepted" reason. It accepts anchors without aliases, `*` inside a comment,
and a quoted `"*a"`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
