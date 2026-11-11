---
id: SR-20261103118152
title: "Code review of quoin PR 785 (retag measurement crates to explicit trace markers)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@dc1e1e7; rust/crates/quoin-measurement, rust/crates/quoin-measurement-graph, rust/Cargo.lock"
review_set: subset
---

# Code review of quoin PR 785

## Summary

Ticket: PLAT-847. Scoped to `git diff origin/main...origin/plat-847-retag-a` at dc1e1e7
(39 files, +307/-261). The PR rewrites 261 `/// Trace: ID` doc lines on tests to
`#[trace("ID", ...)]` markers from `ix_trace_rs`, adds `use ix_trace_rs::trace;` per test
file or test module, and adds `ix-trace-rs` as a dev-dependency of quoin-measurement.
Rust lane (`rust-review`) folded in. Tools: quoin 0.29.4, quire 0.39.0 (engine 0.55.0).

Checks run:

- Fidelity, scripted from git: for every `fn` in the 37 changed `.rs` files, the multiset
  and order of ids on its old `Trace:` line equals the ids in its new `#[trace(...)]`
  marker. 261 markers carry 301 ids; mismatches 0; reordered 0; no fn added, removed or
  renamed; no leftover `Trace:` line on any changed fn.
- Every added marker sits on a `#[test]` fn, none on a helper.
- Doc text outside the `Trace:` clause is unchanged on every fn. The 8 removed lines that
  carried more than ids (checker.rs, execution.rs, six in tc_1947) keep their extra text
  (`Provenance: ...`, the sentence fragment).
- The only other diff lines are the 15 `use ix_trace_rs::trace;` imports, the Cargo.toml
  dev-dependency (`{ workspace = true }`, same as the nine other crates) and one Cargo.lock
  line. No logic change.
- `cargo clippy --tests -p quoin-measurement -p quoin-measurement-graph -- -D warnings`
  passes with and without `--all-features` (the `campaign` feature). `cargo fmt --check` is
  clean for both crates. No CI workflow file changed. Tests not re-run: the attribute
  expands to the item unchanged.
- `quire coverage --json` reproduced on the merge base and on the head: rust bound symbols
  88 -> 349, backed rows 1 -> 2, `trace-id-outside-marker` in these crates 545 -> 242.
- All 85 non-`//!` `trace-id-outside-marker` lines left in these crates were read (the PR
  says 104; the diagnostic count is per id). They are prose cross-references on production
  fns, helpers whose calling test already carries the id, quoted criterion text above a
  test that already has that id in its marker, or analogies (e.g. tc_958_stage_verdicts.rs:1561
  "exactly as a ratchet never holds against one (FR-110-AC-6)"). None of them should be a
  marker, except the `Provenance:` TC ids in FND-002. The 138 `//!` module findings are
  correctly left alone: a module doc has no test symbol to bind.

## Verdict

**PASS WITH FINDINGS (one medium, two low).** The retag is faithful and mechanical: no id
lost, merged, invented or moved to a helper; all feature combinations compile clean. The
findings are about the repo guidance that will undo this work and two small leftovers. Why
backed rows barely moved is recorded in the gap-analysis artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The repo's own Rust guidance still requires the `/// Trace:` doc line, which engine 0.55.0 no longer binds. It also says "there is no hand-written matrix row and no `TC-` id to mint", but the coverage model backs a row only when the test carries its TC id. New tests written to this guidance add unbound legacy tags again. The shipped skill templates teach downstream repos the same form | .claude/skills/rust-style/SKILL.md:178-205, CLAUDE.md:122, skills/spec-matrix/SKILL.md:58, skills/spec-correctness/assets/templates/proptest.rs.md:22, skills/spec-fuzz/assets/templates/cargo-fuzz.rs.md:21, docs/engineering-assurance-adoption.md:58 |
| FND-002 | low | tc_1947 tests keep their matrix row ids (TC-1947, TC-1941) in `Provenance:` prose. The same PR moved TC-1941 and TC-1983 into markers in execution.rs and checker.rs, and spec/matrix.md:1381 cites `tc_1947_two_member_authored_origins_are_recorded` under TC-1947. The `TC-1944` at :544 names a tombstoned row and must not be promoted | rust/crates/quoin-measurement/tests/tc_1947_campaign_input_origins.rs:336, rust/crates/quoin-measurement/tests/tc_1947_campaign_input_origins.rs:385, rust/crates/quoin-measurement/tests/tc_1947_campaign_input_origins.rs:469 |
| FND-003 | low | Removing the `Trace: FR-114-AC-2, TC-1941.` prefix left a short line, "Every EA terminal state maps to its", and the paragraph was not rewrapped | rust/crates/quoin-measurement/src/campaign/run/execution.rs:288 |

## Examined scope

```yaml
clean: false
scope:
- {id: fidelity, path: "37 changed .rs files", role: examined, excerpt: "261 /// Trace: lines -> 261 #[trace(...)] markers, 301 ids, per-fn multiset and order identical"}
- {id: rust/Cargo.lock, path: rust/Cargo.lock, role: examined, excerpt: "+ \"ix-trace-rs\","}
- {id: quoin-measurement/Cargo.toml, path: rust/crates/quoin-measurement/Cargo.toml, role: examined, excerpt: "ix-trace-rs = { workspace = true }"}
- {id: cfg-gating, path: "src/plans/tests.rs, src/campaign/run/domain.rs, tests/tc_1947_campaign_input_origins.rs", role: examined, excerpt: "clippy -D warnings clean with and without --all-features"}
- {id: residual-advisories, path: "quoin-measurement, quoin-measurement-graph", role: examined, excerpt: "85 non-module trace-id-outside-marker lines read; only the tc_1947 Provenance TC ids are trace claims"}
```

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | `tc_1947_selected_input_at_a_tracked_path_is_refused` now carries `TC-1947`. Row TC-1947 (two members' recorded origins) does not cite this test, and the test does not assert it. The test is already correctly tagged `TC-1942`, the row that cites it. Remove `TC-1947` | rust/crates/quoin-measurement/tests/tc_1947_campaign_input_origins.rs:388 |
| FND-005 | low | `tc_1944_no_collection_when_the_producer_yields_none` now carries `TC-1983`. That row is about a checker receipt naming another member, plan or version being reject; the test asserts that a producer with no outputs mints no Collection. This is a wrong row binding. Drop `TC-1983` and leave the honest `criterion-tagged-outside-row` diagnostic, or add a matrix row for the case | rust/crates/quoin-measurement/tests/tc_1947_campaign_input_origins.rs:547 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | 32a4b46 rewrote rust-style, CLAUDE.md, spec-matrix, the docs note, and the first example in each Rust template. These still teach `/// Trace:` for Rust: the rust-style SKILL.md:3 description, proptest.rs.md:37/65/82/103/128, cargo-fuzz.rs.md:69, spec-correctness/references/step-4-generate-tests.md:15,26 and spec-fuzz/references/step-3-emit-targets.md:14 |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-001 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | `tc_166_cargo_fuzz_template_carriers_sit_on_a_named_wrapper` now fails. It counts template lines starting `/// Trace: ` and asserts there is at least one (`"template shows no Trace-carrying wrapper"`). The PR converted cargo-fuzz.rs.md to `#[trace(...)]`, so that count is now 0 (it was 2 on main). FR-038 (line 79) and FR-038-AC-5 also still state the `Trace:` doc-line carrier. Update the test to read the `#[trace("…")]` attribute above the named wrapper, and update the FR-038 text, in this PR | rust/crates/quoin-core/tests/tc_166_spec_fuzz_skill.rs:55-89, skills/spec-fuzz/assets/templates/cargo-fuzz.rs.md:23, spec/functional/FR-038-generate-fuzz-harnesses.md:79 |
| FND-007 | medium | In the FR-018-AC-3 example, the `trace(...)` call sits after `it(...)` in the `describe` body, which is the placement the template itself says binds nothing. With `QUIRE_TRACE_EMIT=1` it also throws `trace() was called outside a running test` at collection. Move the call inside the `it` callback | skills/spec-correctness/assets/templates/fast-check.ts.md:162-163 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | FR-033 still says the trace tags live in source as `/// Trace: FR-002-AC-1`. After this PR that form binds nothing, and the repository writes `#[trace("FR-002-AC-1", ...)]` | spec/functional/FR-033-evidence-format-adapters.md:88 |
