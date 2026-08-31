---
name: gap-analysis
description: Verify a targeted plan is complete and validated — every task done, the Test
  Matrix backed by real tracking tags in tests, and code fully traced to spec (flagging
  underspecified code with no owning requirement). Optional semantic review checks that
  intent↔test↔code actually agree. Emits a quire-validated SpecReview artifact to
  reviews/YY-MM-DD-slug.md.
---

# Gap Analysis

Use this skill as a **post-implementation verification gate** for one targeted plan.
It answers three questions and, on request, a fourth:

1. **Is the plan done?** Every `Task` in the targeted plan bundle is `status: done`.
2. **Is the Test Matrix real?** Every Test Case in the matrix is backed by an actual test
   carrying a matching **tracking tag** (`TC-xxx`, `FR-xxx-AC-x`) in the test code.
3. **Is anything unspecified?** Code/behavior exists with **no owning** StR/US/FR/NFR
   (the reverse, code→spec gap).
4. **(optional) Does intent match reality?** For each requirement↔test↔code triple — does
   the test validate the requirement's *intent*, does the test actually exercise the code,
   and does the code match the requirement's intent.

The result is a single **quire-validated `SpecReview`** document (`analysis: gap-analysis`)
written to `reviews/YY-MM-DD-<slug>.md`, with a **Verdict** (PASS / CONDITIONAL / FAIL)
and a validated **Findings** table.

## When to use

- After `implement-plan` (or any implementation effort) to confirm a plan is genuinely
  complete and verified before closing it.
- To audit drift between spec, tests, and code for an existing component.
- As a release/merge gate that produces a durable, traceable review artifact.

This skill is **read-only over the codebase** — it inspects and reports; it does not fix
code, edit the plan, or change the matrix. Its only write is the SpecReview artifact.

## Inputs

- A target **plan bundle** `plan/<Plan-id>-<slug>/` (the user may name it; otherwise pick).
- The component **spec** (`spec/spec.md` for `org`/`name`) and **Test Matrix**
  (`spec/matrix.md` or `spec/tests.md`).
- The **source** and **test** trees of the component.

## Steps

0.  **[Target selection](references/step-1-target-selection.md)**: Resolve the plan bundle,
    spec root, Test Matrix, and `org`/`component` for `ix://` URIs.
1.  **[Plan completion](references/step-2-plan-completion.md)**: Assert every `Task` is
    `done`; report incomplete/blocked tasks and stale `plan.md` checkboxes.
2.  **[Matrix verification](references/step-3-matrix-verification.md)**: Run
    `quire coverage --scope <root> --json` and interpret the report — unbacked rows, status
    lies, untracked tests, and the backed/total rollup. The reconciliation is the engine's;
    the severity and the verdict stay here. A repo whose module set declares no
    `traceability:` model falls back to a grep index, declared as such.
3.  **[Underspecified code](references/step-4-underspecified-code.md)**: Find code/behavior
    with no owning requirement (reverse gap), plus stubs masquerading as complete.
4.  **[Semantic review](references/step-5-semantic-review.md)** *(OPTIONAL — ask first)*:
    Judge intent↔test↔code agreement per requirement. Skip unless the user opts in.
5.  **[SpecReview artifact](references/step-6-specreview-artifact.md)**: Write and validate
    the `SpecReview` to `reviews/YY-MM-DD-<slug>.md`.

All steps required except Step 4, which is gated on user choice.

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

Steps 1–3 are mechanical and cheap. Step 4 is an expensive, judgment-heavy LLM pass.
**Before running it**, ask the user explicitly (e.g. with a yes/no choice):

> Run the optional semantic review (intent↔test↔code)? It's slower but verifies that tests
> actually validate requirement intent and exercise real code.

If yes, fan the work out (one subagent per FR or per area) for thoroughness. If no, note in
the SpecReview's Coverage section that semantic review was skipped.

## Output

A `SpecReview` (`spec-artifacts-process` archetype) at `<project_root>/reviews/YY-MM-DD-<slug>.md`:

- Frontmatter `type: SpecReview`, `analysis: gap-analysis`, `id: SR-NNN`, `scope`,
  `review_set: subset`, and `relationships:` (`reviews` → the plan, `references` → the matrix).
- Body: `## Summary`, `## Verdict` (PASS/CONDITIONAL/FAIL), `## Findings`
  (validated table `ID | Severity | Summary | Refs`), `## Coverage` (rollup).

> **Note:** `reviews/` is at the **repo root** by deliberate choice for this skill, not
> `spec/reviews/`. quire validation is path-agnostic, so this is fine.

Validate before finishing:

```
quire validate --scope <project_root> "reviews/**/*.md"
```

## Verdict rule

- **FAIL** — any incomplete/blocked task, any matrix Test Case with no backing tagged test,
  or any `high`-severity finding.
- **CONDITIONAL** — only `medium`/`low` findings (e.g. untracked tests, minor drift).
- **PASS** — no gaps; record the single `FND-001 | low | No gaps found | -` row.
