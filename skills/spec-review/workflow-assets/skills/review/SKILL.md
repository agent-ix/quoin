---
name: spec-review
description: Run a composite specification review.
contributes:
  workflows: ./workflows
---

# /spec-review

## When To Use

Use this skill for a full or multi-artifact spec review before implementation, release, or migration.

## What It Does

Run a composite specification review. The workflow definition is the source of truth for
phases, gates, item schemas, and recipes. The review produces **one validated `SpecReview`
document per analysis** under `spec/reviews/`, rather than a single freeform report.

## Workflow Behavior

- Start from the workflow status and follow reported next actions.
- **intake — offer the review set.** Before advancing, present the choice to the user and
  inspect the requested scope for an applicable installed `AssuranceProfile`. Record its
  path, `review_selection.mode`, and profile analyses when present, then record
  `review_set` plus `selected_analyses`:
  - `base` — the skill checklist only (ID formats, US/FR/TC quality, the six coverage
    rules). No analysis skills. `selected_analyses` is empty.
  - `all` — `base` plus all seven analyses: `failure-domain`, `integrity`, `dependency`,
    `evidence`, `risk-complexity`, `scope-boundary`, `ears-conformance`.
  - `subset` — `base` plus the analyses the user picks from those seven.
  - A profile `recommend` selection is a default the user may change. A profile `require`
    selection must use `subset` with `selected_analyses` exactly equal to
    `profile_analyses`; the intake invariant rejects any bypass.
  - If the profile names an analysis outside the installed seven-value contract, stop
    and report the unavailable schema/skill dependency. Do not author an invalid review.
- **analyses_run — run the selected set, one SpecReview doc each.** Run each selected
  analysis (the matching `spec-*-analysis` skill). Prefer running them **in parallel** —
  each writes its own file, so there is no contention.
  - **Guardrail — fetch the template from quoin (do not invent the format).** Once, run
    `quoin write <repo> --types SpecReview` and use the emitted skeleton + schema as the
    authoritative contract. Reuse that one contract for every analysis doc.
  - **Direct-author** a `SpecReview` markdown document per analysis to
    `spec/reviews/<analysis>.md` from that template. `base` renders a single
    `spec/reviews/base.md` with `analysis: base`.
- **reviews_rendered — validate (guardrail).** Always run the quire validation command (the
  calling tool emits it, e.g. `quire validate --scope <repo> "spec/**/*.md"`). Fix any finding-table / severity
  / id errors it reports. Only once a SpecReview doc validates, record it:
  `add-item review_doc --item '{"id":"RD-<analysis>","analysis":"<analysis>","path":"spec/reviews/<analysis>.md"}'`
  (the item `id` must be non-empty). These items drive the coverage gate.
- Use recipes for deterministic command chains, but preserve the human gate for acceptance.

## SpecReview document structure

The **authoritative** template comes from `quoin write --types SpecReview` (fetch it; do not
hand-copy). The structure below is a reference for what that skeleton contains — each
`spec/reviews/<analysis>.md` is a `SpecReview` artifact (archetype in
`spec-artifacts-process`):

```markdown
---
id: SR-001
title: "<analysis> review of <scope>"
type: SpecReview
analysis: <failure-domain|integrity|dependency|evidence|risk-complexity|scope-boundary|ears-conformance|base>
scope: <spec paths / ids>
review_set: <base|all|subset>
---

## Summary

<one or two sentences on what this analysis found>

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | <one-line finding> | FR-014 |
```

The `## Findings` table is **validated by quire**: exact columns `[ID, Severity, Summary,
Refs]`, at least one row, `FND-NNN` ids, and `Severity` must be one of `low`/`medium`/`high`.
An analysis that found nothing still renders a doc; record that as a single
`FND-001 | low | No issues found | —` row (or state it in `## Summary`).

## Coverage gate

The final `validated → accepted` transition enforces `selected_analyses_covered`: the run
cannot be accepted until every selected analysis has a recorded `review_doc`. If it reports
`selected_analyses_not_run` with a `missing` list, render + validate + record those analyses
before retrying acceptance. `base` passes with no analyses required.
The intake transition separately enforces `review_selection_consistent`, including exact
all-set expansion and required-profile selection. A recommendation never silently
becomes enforcement.

## Acceptance Criteria

- AC-1: Reviews structural integrity, traceability, coverage, and readiness across selected artifacts.
- AC-2: Produces one validated `SpecReview` doc per selected analysis before acceptance.
- AC-3: Stops at acceptance with findings and recommended next workflows rather than making broad edits.

## Boundaries

- Do not silently overwrite existing human-authored requirements artifacts.
- Do not invent missing facts, links, or evidence; record assumptions and gaps.
- Do not expand to adjacent spec operations unless the user request requires it or the workflow directs it.
