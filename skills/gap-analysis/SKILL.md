---
name: gap-analysis
description: >-
  Audit a repository end to end — spec requirements, the computed Test Matrix, tagged
  tests, and source code. Planless by default; it reads `quoin matrix` / `quire matrix` to
  find criteria no test is tagged with, finds code with no owning requirement, and catches stubs and coverage inflation, reporting
  "Plan completion: not assessed". An explicitly supplied plan adds optional
  plan-completeness bookkeeping only. Optional semantic review checks whether intent, test
  and code agree. Emits a quire-validated SpecReview artifact to reviews/YY-MM-DD-<slug>.md.
---

# Gap Analysis

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

Use this skill as a **repository assurance audit**. The chain it verifies is always the
same, with or without a plan:

```
spec criteria  ↔  computed Test Matrix  ↔  tagged tests  ↔  source code
```

It answers three questions by default, a fourth on request, and a fifth only when the
caller explicitly hands it a plan:

1. **Is every criterion tested?** Every acceptance criterion is backed by an actual test
   carrying its id as a **trace tag** (`FR-xxx-AC-x`) in the test code. The Test Matrix
   that answers this is computed by `quire matrix` (and `quoin matrix`, which adds run
   evidence); nobody writes it by hand.
2. **Is anything unspecified?** Code/behavior exists with **no owning** StR/US/FR/NFR
   (the reverse, code→spec gap).
3. **Is the evidence hollow?** Source or test stubs, and coverage inflation, standing
   behind a tagged criterion.
4. **(optional, opt-in) Does intent match reality?** For each requirement↔test↔code triple —
   does the test validate the requirement's *intent*, does the test actually exercise the
   code, and does the code match the requirement's intent.
5. **(only with an explicit plan) Is that plan complete?** Every `Task` is `status: done`,
   done tasks did not complete out of dependency order, and `plan.md` checkboxes agree with
   task status.

The result is a single **quire-validated `SpecReview`** document (`analysis: gap-analysis`)
written to `reviews/YY-MM-DD-<slug>.md`, with a **Verdict** (PASS / CONDITIONAL / FAIL)
and a validated **Findings** table.

## Modes

**Planless (the default).** No plan is selected, discovered, or asked for. The repository
audit — matrix verification, reverse gaps, and stubs — runs in full, and the semantic review
is offered on the same terms as ever. The SpecReview records, verbatim:

```
Plan completion: not assessed
```

**Plan-assisted (opt-in).** The caller names a plan — `gap-analysis --plan Plan-001`, or
says so in the request. The same repository audit runs **identically and over the same
repository scope**, and plan-completeness bookkeeping is added on top.

A plan is an *addition*, never a lens. It does not choose the scope, does not narrow the
requirement set, does not bound the code surface inventoried, and does not suppress a
finding for sitting outside its tasks.

## Non-negotiables

- **Never author.** A gap-analysis run does not create or edit a plan, a `Task`, a
  requirement, or an acceptance criterion. Its only write is the SpecReview artifact.
- **Never manufacture a plan to satisfy the audit.** An absent plan bundle is the default
  case, not a blocker. Do not offer to run `spec-to-plan` first as a precondition.
- **Never auto-select a plan.** A `plan/` directory containing bundles does not make this a
  plan-assisted run. Only an explicit caller instruction does.
- **Never write a Test Matrix.** The matrix is computed from criteria and trace tags. A
  hand-written `spec/matrix.md` or `spec/tests.md`, where one still exists, is not audited,
  and its absence is not a finding.
- **A planless PASS is a repository claim only.** It means traceability, reverse-gap, and
  stub checks passed. It never means the work was planned, tracked, or completed against a
  plan. Do not let the Summary or Verdict imply otherwise.

## When to use

- To audit drift between spec, tests, and code for any existing component — including one
  whose work predates planning, or whose scope was never organized as a plan.
- As a release/merge gate that produces a durable, traceable review artifact.
- After `implement-plan`, with `--plan <Plan-id>`, to additionally confirm that plan is
  genuinely complete.

This skill is **read-only over the codebase** — it inspects and reports; it does not fix
code, edit the plan, or add trace tags.

## Inputs

- The **repository** under audit (required): its spec root, source tree, and test tree.
- The component **spec** (`spec/spec.md` for `org`/`name`, and its criteria).
- *Optional:* a **plan bundle** `plan/<Plan-id>-<slug>/`, only when the caller names one.

## Steps

Each step says when it runs. Nothing here is numbered, because the plan step is an overlay
rather than a position in a sequence, and a number would imply the audit waits on it.

**Always — [Target selection](references/step-1-target-selection.md).** Resolve the
repository root, spec root, and `org`/`component` for `ix://` URIs. Record
whether a plan was explicitly supplied.

**Always — [Matrix verification](references/step-3-matrix-verification.md).** Run
`quoin matrix --repo <root> --json` (or `quire matrix --scope <root> --format json` when
there is no evidence to read) and interpret the computed matrix — untagged criteria,
criteria tagged only by ignored tests, suspect or stale evidence, and stale tags. The
computation is the engine's; the severity and the verdict stay here. A repo whose module
set declares no `traceability:` model falls back to a grep index, declared as such.

**Always — [Underspecified code](references/step-4-underspecified-code.md).** Find
code/behavior with no owning requirement (reverse gap), plus stubs and coverage inflation
masquerading as complete.

**Only when the user opts in — [Semantic review](references/step-5-semantic-review.md).**
Judge intent↔test↔code agreement per requirement. Ask first; skip unless the user says yes.
Opt-in works the same way in both modes.

**Only with an explicit plan — [Plan completeness](references/step-2-plan-completion.md).**
Assert every `Task` is `done`, check done-task dependency order, and reconcile `plan.md`
checkboxes against task status. This is bookkeeping added to the audit, not a gate in front
of it.

**Always — [SpecReview artifact](references/step-6-specreview-artifact.md).** Write and
validate the `SpecReview` to `reviews/YY-MM-DD-<slug>.md`.

> **`--scope` is the repository root, and must be passed explicitly.** The command derives **two roots** from it and never
> interchanges them: spec documents are read from `<repo>/spec` only, trace tags from
> the source tree at `<repo>` excluding `spec/`. A repo with no `spec/` exits with a
> diagnostic naming the missing document root rather than scanning the whole tree, and a
> document outside `spec/` (a fixture, a `plan/` copy) mints nothing. A relative glob
> resolves under `--scope` only in scoped mode (no `--module`); with `--module` it
> resolves against the process working directory, and an omitted `--scope` defaults to
> `.` — so a run launched from a parent directory validates the **wrong tree** and exits
> 0 for whatever it matched.

## The optional semantic review

The matrix, reverse-gap, and plan-completeness steps are mechanical and cheap. The semantic
review is an expensive, judgment-heavy LLM pass. **Before running it**, ask the user
explicitly (e.g. with a yes/no choice):

> Run the optional semantic review (intent↔test↔code)? It's slower but verifies that tests
> actually validate requirement intent and exercise real code.

If yes, fan the work out (one subagent per FR or per area) for thoroughness. If no, note in
the SpecReview's Coverage section that semantic review was skipped.

## Output

A `SpecReview` (`spec-artifacts-process` archetype) at `<project_root>/reviews/YY-MM-DD-<slug>.md`:

- Frontmatter `type: SpecReview`, `analysis: gap-analysis`, `id: SR-NNN`, `scope`,
  `review_set: subset`, and `relationships:` (`references` → the spec root; plus `reviews`
  → the plan **only** in plan-assisted mode).
- Body: `## Summary`, `## Verdict` (PASS/CONDITIONAL/FAIL), `## Findings`
  (validated table `ID | Severity | Summary | Refs`), `## Coverage` (rollup, including the
  plan-completion line).

> **Note:** `reviews/` is at the **repo root** by deliberate choice for this skill, not
> `spec/reviews/`. quire validation is path-agnostic, so this is fine.

Validate before finishing:

```
quire validate --scope <project_root> "reviews/**/*.md"
```

## Verdict rule

- **FAIL** — any untagged criterion, any `high`-severity finding,
  or (plan-assisted only) any incomplete/blocked task.
- **CONDITIONAL** — only `medium`/`low` findings (e.g. untracked tests, minor drift).
- **PASS** — no gaps; record the single `FND-001 | low | No gaps found | -` row.

A planless run can reach PASS. That PASS asserts repository assurance — criterion traceability,
reverse-gap, and stub checks — and nothing about whether the work was ever planned. The
`Plan completion: not assessed` line in `## Coverage` is what keeps the two apart, and it is
mandatory.
