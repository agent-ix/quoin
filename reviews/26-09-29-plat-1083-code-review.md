---
id: SR-176
title: "Code review — PLAT-1083 skills and templates stop hand-writing the Test Matrix"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@1aceb1552747024441f1ff7ddc1cd024d5a2a21f; PR 653 diff vs origin/main d79c3b9 (docs review): .claude/skills/rust-style/SKILL.md, README.md, skills/gap-analysis/{SKILL.md,references/step-1,2,3,4,6}, skills/spec-app-review/references/checklist.md, skills/spec-correctness/{SKILL.md,references/step-0,2,3,4,6,7}, skills/spec-criterion-strength-analysis/SKILL.md, skills/spec-evidence-analysis/SKILL.md, skills/spec-matrix/{SKILL.md, assets deleted}, skills/spec-object-review/references/object-type-guide.md, skills/spec-review/references/checklist.md, skills/specify/{SKILL.md,assets/spec-template.md,references/writing-good-requirements.md}, templates/semantic-module/{conformance.yaml,hooks/pre_gen_project.py,{{cookiecutter.repo_name}}/{AGENTS.md,CLAUDE.md,CONTRIBUTING.md,README.md,spec/index.md,spec/matrix.md deleted,spec/spec.md}}"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-082"
    type: "reviews"
---

# SR-176: Code review — PLAT-1083 skills and templates stop hand-writing the Test Matrix

## Summary

Ticket: PLAT-1083 (epic PLAT-1076). PR agent-ix/quoin#653 at `1aceb15`. A docs review of every skill, the
repo rust-style skill, and the semantic-module template changed by the PR. No Rust source changed, so the
`rust-review` lane has nothing to review. Grep over `skills/`, `.claude/skills/` and `templates/` for
`matrix.md`, `tests.md`, `test-cases/`, `TC-`, `Test Case Summary`, `Traces To`, status markers and
`spec-matrix` finds no remaining instruction to write matrix rows, TC ids or Status markers. The survivors are
all accepted or benign: spec-to-plan's plan-local TC ids, gap-analysis step-2's `TC-004 verifies` Task-contract
example (the same spec-to-plan contract), fixture JSON, an app-spec trust-boundary table, and the
spec-integrity-analysis requirement-traceability deliverable, which is not a test matrix.

Every command and flag the skills cite was checked against source. For `quire matrix` (quire-cli `v0.34.0`,
`c739677`, `src/commands/matrix.rs`), `--scope`, `--format markdown|json|tsv` and `--strict` exist. The status
tokens are `tagged`, `untagged`, `tagged-by-ignored-test` and `method-without-symbol`. Each binder carries
`path/line/column/qualified_name/kind` plus `ignored`. For `quoin matrix` (this worktree,
`rust/crates/quoin-cli/src/matrix.rs`), `--repo` (default `.`) and `--json` exist. `MatrixCriterion` carries
`static_status`, `binders`, `method`, `evidence_status` (`bound|stale|suspect|undischarged|no run evidence`)
and `evidence_detail{findings,bindings,unevaluated}`, and an empty matrix carries `reason`. `quire coverage
--json` still emits `untracked_symbols` and `diagnostics`. `@pytest.mark.trace("…")` is the
`pytest-trace-marker` declared in spec-artifacts-process `manifest.yaml` `trace_tags.markers`. `Trace:` lines
bind through the declared `legacy` forms. The release binary `quire 0.34.0` ran `quire matrix --format tsv`
and `quire coverage --json` successfully on this worktree.

## Verdict

**PASS with low findings**. No high or medium findings. The intent is met in the skills and the template. The
low findings are precision gaps in the new guidance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | gap-analysis step 3 invokes `quire matrix --format json`, but it says the zero-criteria signal is the printed line `No obligations matched this scope.`. That line is rendered by the markdown format only. Under `--format json` the `coverage_matrix` key is absent (quire-cli `matrix.rs` `Payload` `skip_serializing_if = Vec::is_empty`). An agent that follows the step and looks for the string never sees it. | skills/gap-analysis/references/step-3-matrix-verification.md:100 |
| FND-002 | low | The skills gate `quire --version >= 0.34.0`, but they cite `quoin matrix --repo <root> --json` with no quoin version gate. FR-115 (`3fa9921`) is not in any tag. On the released quoin 0.24.1/0.25.0, `quoin matrix` is the ix-flow launcher "Build or update a requirements test matrix", which has no `--repo` flag. gap-analysis falls back when the command "is unavailable or refuses". spec-matrix step 1 has no fallback. | skills/spec-matrix/SKILL.md:26, skills/gap-analysis/references/step-3-matrix-verification.md:23-28 |
| FND-003 | low | spec-matrix's Rust example teaches `/// Trace: FR-012-AC-3` and never names the declared Rust marker `#[trace("…")]`. The module manifest classes `Trace:` as a `legacy` form with `rewrite_to: rust-trace-attribute`. Owner intent says `Trace:` lines bind, and "the form your repository already uses" hedges it, so this is not a defect. A new repository following the skill still starts on the legacy form. | skills/spec-matrix/SKILL.md:43-47 |

## Dispositions

Round 1, reviewed at `86f7c36cf8b592c59c106074f42f82b283f1eaf5`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 86f7c36: step 3 now says that under `--format json` the `coverage_matrix` key is absent and the "No obligations matched" line is markdown-only. This matches quire-cli `matrix.rs` `Payload`. |
| FND-002 | fixed | 86f7c36: both skills now say `quoin matrix --repo` needs the first release after 0.25.0 that carries FR-115. spec-matrix adds a fallback to `quire matrix --format json`. |
| FND-003 | fixed | 86f7c36: the Rust example is now `use ix_trace_rs::trace;` plus `#[trace("FR-012-AC-3")]`, with a dev-dependency note. That matches quire-rs usage: `ix-trace-rs` under `[dev-dependencies]`, and `tests/assurance_boundary.rs` imports `ix_trace_rs::trace`. The multi-id form `#[trace("A", "B")]` matches the manifest `rust-trace-attribute` pattern, and `Trace:` is described as the legacy form. The extra `no_source_symbol` list `[Eval, Manual, Inspection, Analysis]` matches spec-artifacts-process v0.27.0 `manifest.yaml:1432` exactly. |

Round 2, reviewed at `a50fd540ad9285ef00bff1513076f98827ab8c8e`: no finding in this file was open, so no row was added. There is no regression in this file's scope. `quire validate` 0.34.0 on FR-082 exits 0. The only `spec/evals.md` error, at line 79, is identical on `origin/main`.
