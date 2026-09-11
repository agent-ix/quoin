---
name: gap-analysis
description: >-
  Audit a repository end to end — spec requirements, Test Matrix, tagged tests, and source
  code. Planless by default; it verifies the matrix is backed by real tracking tags, finds
  code with no owning requirement, and catches stubs and coverage inflation, reporting
  "Plan completion: not assessed". An explicitly supplied plan adds optional
  plan-completeness bookkeeping only. Optional semantic review checks whether intent, test
  and code agree. Emits a quire-validated SpecReview artifact to reviews/YY-MM-DD-<slug>.md.
---

# Gap Analysis

Use this skill as a **repository assurance audit**. The chain it verifies is always the
same, with or without a plan:

```
spec requirements  ↔  Test Matrix  ↔  tagged tests  ↔  source code
```

It answers three questions by default, a fourth on request, and a fifth only when the
caller explicitly hands it a plan:

1. **Is the Test Matrix real?** Every Test Case in the matrix is backed by an actual test
   carrying a matching **tracking tag** (`TC-xxx`, `FR-xxx-AC-x`) in the test code.
2. **Is anything unspecified?** Code/behavior exists with **no owning** StR/US/FR/NFR
   (the reverse, code→spec gap).
3. **Is the evidence hollow?** Source or test stubs, and coverage inflation, standing
   behind a ✅ row.
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
audit — matrix verification, reverse gaps, stubs, and the opt-in semantic review — runs in
full. The SpecReview records, verbatim:

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
- **A missing Test Matrix stays a `high` finding.** Do not invent one, and do not downgrade
  it because the repository never had a plan either.
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
code, edit the plan, or change the matrix.

## Inputs

- The **repository** under audit (required): its spec root, source tree, and test tree.
- The component **spec** (`spec/spec.md` for `org`/`name`) and **Test Matrix**
  (`spec/matrix.md` or `spec/tests.md`).
- *Optional:* a **plan bundle** `plan/<Plan-id>-<slug>/`, only when the caller names one.

## Steps

0.  **[Target selection](references/step-1-target-selection.md)**: Resolve the repository
    root, spec root, Test Matrix, and `org`/`component` for `ix://` URIs. Record whether a
    plan was explicitly supplied.
1.  **[Matrix verification](references/step-3-matrix-verification.md)**: Run
    `quire coverage --scope <root> --json` and interpret the report — unbacked rows, status
    lies, untracked tests, and the backed/total rollup. The reconciliation is the engine's;
    the severity and the verdict stay here. A repo whose module set declares no
    `traceability:` model falls back to a grep index, declared as such.
2.  **[Underspecified code](references/step-4-underspecified-code.md)**: Find code/behavior
    with no owning requirement (reverse gap), plus stubs and coverage inflation
    masquerading as complete.
3.  **[Semantic review](references/step-5-semantic-review.md)** *(OPTIONAL — ask first)*:
    Judge intent↔test↔code agreement per requirement. Skip unless the user opts in. Opt-in
    works the same way in both modes.
4.  **[Plan completeness](references/step-2-plan-completion.md)** *(ONLY with an explicit
    plan)*: Assert every `Task` is `done`, check done-task dependency order, and reconcile
    `plan.md` checkboxes against task status.
5.  **[SpecReview artifact](references/step-6-specreview-artifact.md)**: Write and validate
    the `SpecReview` to `reviews/YY-MM-DD-<slug>.md`.

Steps 0–2 and 5 always run. Step 3 is gated on user opt-in. Step 4 runs only when the
caller supplied a plan.

> **`--scope` is the repository root, and must be passed explicitly.** Since quire-cli
> v0.16.0 (quire-rs CR-045) the command derives **two roots** from it and never
> interchanges them: spec documents are read from `<repo>/spec` only, trace tags from
> the source tree at `<repo>` excluding `spec/`. A repo with no `spec/` exits with a
> diagnostic naming the missing document root rather than scanning the whole tree, and a
> matrix outside `spec/` (a fixture, a `plan/` copy) mints nothing. A relative glob
> resolves under `--scope` only in scoped mode (no `--module`); with `--module` it
> resolves against the process working directory, and an omitted `--scope` defaults to
> `.` — so a run launched from a parent directory validates the **wrong tree** and exits
> 0 for whatever it matched. Check `quire --version` ≥ 0.16.0 before relying on any of
> this; ≤ 0.15.0 has the pre-split traversal semantics.

## The optional semantic review

The matrix, reverse-gap, and plan-completeness steps are mechanical and cheap. The semantic review is an expensive,
judgment-heavy LLM pass. **Before running it**, ask the user explicitly (e.g. with a yes/no
choice):

> Run the optional semantic review (intent↔test↔code)? It's slower but verifies that tests
> actually validate requirement intent and exercise real code.

If yes, fan the work out (one subagent per FR or per area) for thoroughness. If no, note in
the SpecReview's Coverage section that semantic review was skipped.

## Output

A `SpecReview` (`spec-artifacts-process` archetype) at `<project_root>/reviews/YY-MM-DD-<slug>.md`:

- Frontmatter `type: SpecReview`, `analysis: gap-analysis`, `id: SR-NNN`, `scope`,
  `review_set: subset`, and `relationships:` (`references` → the matrix; plus `reviews` →
  the plan **only** in plan-assisted mode).
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

- **FAIL** — any matrix Test Case with no backing tagged test, any `high`-severity finding,
  or (plan-assisted only) any incomplete/blocked task.
- **CONDITIONAL** — only `medium`/`low` findings (e.g. untracked tests, minor drift).
- **PASS** — no gaps; record the single `FND-001 | low | No gaps found | -` row.

A planless run can reach PASS. That PASS asserts repository assurance — matrix traceability,
reverse-gap, and stub checks — and nothing about whether the work was ever planned. The
`Plan completion: not assessed` line in `## Coverage` is what keeps the two apart, and it is
mandatory.
