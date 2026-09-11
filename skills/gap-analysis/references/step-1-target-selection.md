# Target Selection

**Goal**: Resolve exactly what this run audits — one **repository**, its spec, its Test
Matrix, its source and test trees — and the `ix://` identity used in the output artifact.
Record whether the caller explicitly supplied a plan.

The target of gap-analysis is a repository, not a plan. A plan is an optional extra input.

## Process

### 1. Resolve the repository scope

1. **Repository root.** The repo under audit (`--scope <project_root>` for every `quire`
   invocation later). Default to the repository the session is working in unless the user
   names another.
2. **Spec root.** Single-repo: `spec/`. Multi-repo: `specs/<category>/<component>/spec/`.
3. **`org` / `component`.** Read from `spec/spec.md` frontmatter (`org`, `name`). These build
   the `ix://<org>/<component>/<id>` URIs used in the SpecReview `relationships:`.
4. **Test Matrix.** Look for `spec/matrix.md` first, then `spec/tests.md` (both names are in
   use across the ecosystem). Note its frontmatter `id` (e.g. `TestMatrix-001` / `TM-001`)
   for the `references` edge.
5. **Requirements.** Note the requirement files under `spec/functional/`,
   `spec/non-functional/`, `spec/usecase/`, `spec/stakeholder/` — the matrix and reverse-gap
   steps trace against these ids.
6. **Source / test trees.** Identify where implementation and tests live (e.g. `src/` +
   `tests/`, or language-specific layout).

### 2. Determine the mode

**Planless is the default.** Do not glob `plan/`, do not present a plan menu, and do not ask
which plan to audit. The run is plan-assisted **only** when the caller explicitly names a
plan — `--plan Plan-001`, "gap-analyse Plan-002", or a plan established as the target earlier
in the session.

When a plan *is* supplied:

- Resolve its bundle path `plan/<Plan-id>-<slug>/` and confirm it exists. If the named plan
  does not exist, say so and ask — do not silently fall back to planless, and do not
  substitute a different bundle.
- Record the `<Plan-id>` for the SpecReview `reviews` edge and the plan-completeness step.
- **The bundle does not change anything resolved in section 1.** Scope stays the repository.
  The requirement set stays the full spec. The code surface stays the whole source tree.

### 3. Hand off

State explicitly, so later steps and the artifact agree:

- repository root, spec root, matrix path + id
- `ix://<org>/<component>` prefix
- where source and tests live
- **mode**: `planless` or `plan-assisted (<Plan-id>)`

## Notes

- If there is **no** plan bundle, that is the normal case. Proceed planless; the SpecReview
  will record `Plan completion: not assessed`. Never tell the user to run `spec-to-plan`
  first, and never author a retrospective plan to make the audit possible — that reverses
  the assurance order the skill exists to enforce.
- If there is **no** Test Matrix, the remaining steps still run, but record a `high` finding
  that the matrix is missing and that coverage cannot be verified. Do not create a matrix.
