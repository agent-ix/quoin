---
name: spec-to-plan
description: Convert ISO spec requirements (StR, FR, NFR) into a TDD-based project plan with dependency analysis, parallel execution tracks, quality gates, and task decomposition. Selects or starts a plan and writes a frontmatter-typed plan bundle under plan/<Plan-id>-<slug>/ (Plan + Task + index + log).
---

# Spec to Plan

This skill provides a structured workflow for converting ISO/IEC/IEEE 29148 compliant specifications into an executable, dependency-aware project plan with parallel execution tracks and quality gates.

A project may hold **multiple plans** — each a named group of tasks (e.g. "core app",
"packaging", "a11y hardening"). Every plan is a self-describing **bundle** under
`plan/<Plan-id>-<slug>/` containing an `index.md`, a `log.md`, the `plan.md` itself,
and a `tasks/` directory. Plans, tasks, and the index/log files are
**frontmatter-typed** (`Plan`, `Task`, `index`, `log`) so the dependency graph and
all task metadata are machine-parseable and quire-validatable.

## Purpose

Use this skill when:
- You are starting a new implementation effort based on an existing Spec.
- You want to add a *new* plan to a project that already has one or more.
- You need to generate a Test Plan before writing code (TDD).
- You want to ensure full traceability from Requirements (StR/FR/NFR) to Tests.
- You need to identify what can be parallelized vs what must be serial.
- You want quality gates to catch fundamental problems before downstream work is wasted.

## Pre-conditions (Readiness Gate)

Task generation MAY begin only when ALL of the following are true:

- All requirements are atomic and testable.
- Ownership is assigned (see `spec-scope-boundary-analysis`).
- Dependencies are known (see `spec-dependency-analysis`).
- Risks are classified (see `spec-risk-complexity-analysis`).
- Verification strategy is defined (see `spec-evidence-analysis`).
- Failure-domain analysis is complete (see `spec-failure-domain-analysis`).

Failure to meet these conditions WILL result in unstable tasking. If any condition is unmet, stop and complete the corresponding analysis skill first.

## Steps
All steps required!

0.  **[Plan Selection](references/step-0-plan-selection.md)**: Select an existing plan to continue, or start a new one. Skip only if the target plan is already fixed in this session's context. Scaffolds the bundle (`index.md`, `log.md`, `tasks/`).
1.  **[Analysis](references/step-1-analysis.md)**: Analyze the spec structure and map dependencies into the bundle's `plan.md`.
2.  **[Test Plan](references/step-2-test-plan.md)**: Generate the list of required tests.
3.  **[Execution Plan](references/step-3-execution-plan.md)**: Derive dependency graph, critical path, parallel tracks, quality gates, and task decomposition into frontmatter-typed `Task` files.

## Output

This skill produces or updates a **plan bundle** at `<project_root>/plan/<Plan-id>-<slug>/`:

-   `plan.md` (`type: Plan`): The central living document — requirements summary,
    dependency graph, execution tracks, quality gates, and test plan. It is an
    **orchestration overview, not a mirror of the bundle**: ownership, the DAG, and test
    traces are authoritative in the task frontmatter, so `plan.md` states each fact once
    (see `references/step-3-execution-plan.md`). Frontmatter `relationships:` trace
    (`references`) the requirements the plan covers.
-   `tasks/Task-NNN-<slug>.md` (`type: Task`): One file per task. Frontmatter carries
    the machine contract — `status`, `track`, `priority`, and `relationships:` edges
    (`depends_on` for the DAG, `references` for requirement ownership, `verifies` for
    test traces). The body holds the human sections (Scope/Subtasks/Deliverables/Notes).
-   `index.md` (`type: index`): `## Contents` link list to `plan.md` and every task —
    the bundle's table of contents.
-   `log.md` (`type: log`): `## History` of dated plan lifecycle events (plan created,
    tasks added, status changes).

Validate the bundle with:
`quire validate --scope <project_root> "plan/**/*.md"`

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


## Open in the review cockpit

After the bundle validates, run `filament open <plan-bundle>/plan.md` to open the
plan in the running filament-ide cockpit (it fires the `filament-ide://` deep link
and prints a clickable link as a fallback). Skip silently if the `filament`
command is not installed.
