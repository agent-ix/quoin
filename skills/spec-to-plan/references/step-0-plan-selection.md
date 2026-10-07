# Step 0: Plan Selection

**Goal**: Decide which plan this run targets — continue an existing one or start a
new one — and scaffold the plan bundle so Steps 1–3 have a place to write.

A project holds **multiple plans** under `plan/`, each a bundle
`plan/<Plan-id>-<slug>/`. This step picks exactly one target bundle.

## Process

### 1. Resolve the target plan

1. **Already in context?** If the user named a plan (an arg like `Plan-002`, or a
   plan established earlier this session), use it and skip the prompt.
2. **Discover existing plans.** Glob `plan/*/plan.md`. For each, read the frontmatter
   `id`, `title`, and `status`. Present them:

   ```
   Existing plans:
   - Plan-001  electron-hello core app        (active)
   - Plan-002  packaging & distribution        (active)

   Continue one of these, or start a new plan?
   ```
3. **Continue** (update an existing plan, e.g. after a spec change) → use the chosen
   bundle and **update it in place** (see below). **New** → go to step 2 below.

#### Updating an existing plan

A common flow is: the spec changed (a requirement was added, edited, or removed),
now regenerate the plan. Continue the existing bundle — **never spawn a second plan
for the same effort**:

- **Diff coverage.** Re-read the current requirements and compare them to the plan's
  `relationships: references` and existing `tasks/`. Identify requirements that are
  new, changed, or removed.
- **Add only what's missing.** Create `Task` files for newly covered requirements,
  continuing the existing `Task-NNN` numbering. Do **not** duplicate tasks that
  already exist for unchanged requirements.
- **Reconcile changed/removed.** For a changed requirement, update its task's scope
  and `verifies`/`references` edges. For a removed requirement, drop or mark its task
  (don't silently leave an orphan).
- **Preserve progress.** Leave `status` on tasks already `in_progress`/`done` intact.
- **Update the plan doc + edges.** Extend `plan.md`'s `relationships: references` and
  body (dependency graph, tracks) to include the new work.
- **Refresh index.** Update `index.md`'s `## Contents` for any added/removed
  task files.

### 2. Create a new plan bundle

1. **Allocate the id.** Next `Plan-NNN` after the highest existing id (zero-padded
   to 3 digits, e.g. `Plan-003`). If none exist, `Plan-001`.
2. **Derive `slug` and `title`.** Slug is kebab-case, no id prefix (e.g.
   `electron-hello-core`). Title is human-readable.
3. **Scope the requirements.** Collect the requirement ids (StR/FR/NFR) this plan
   covers — these become the plan's `relationships: references` edges.
4. **Scaffold the bundle** at `plan/<Plan-id>-<slug>/`:

   - `plan.md` — `type: Plan` frontmatter (Step 1 fills the body):

     ```yaml
     ---
     id: Plan-003
     title: "Packaging & distribution"
     type: Plan
     status: active
     relationships:
       - target: ix://<org>/<component>/FR-008
         type: references
       # …one per covered requirement
     ---
     ```
     `<org>`/`<component>` come from the spec's `spec.md` frontmatter (`org`, `name`).

   - `index.md` — `type: index`, body `## Contents` (link list; starts with `plan.md`,
     grows as tasks are added in Step 3):

     ```markdown
     ---
     type: index
     title: "Plan-003 — Packaging & distribution"
     description: "Contents of the Plan-003 bundle."
     okf_version: "0.1"
     ---
     # Plan-003 — Packaging & distribution

     ## Contents

     * [Plan-003: Packaging & distribution](./plan.md) - Plan overview, DAG, tracks, gates.
     ```

   - `tasks/` — empty directory (Step 3 populates it).

### 3. Hand off

State the resolved bundle path (e.g. `plan/Plan-003-packaging-distribution/`)
explicitly so Steps 1–3 — and any downstream skill — write to the right place.

## Notes

- **Never write to a second plan's files** during this run. One run targets one bundle.
- `index.md` follows the reserved OKF archetype from `spec-artifacts-iso`
  (`skeletons/index.md`).
