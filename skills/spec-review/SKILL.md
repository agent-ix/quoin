---
name: spec-review
description: Review requirements for quality, consistency, and completeness.
---

# Review Requirements

Use this skill to validate requirements artefacts before implementation.

## Checklist

See **[references/checklist.md](references/checklist.md)** for the detailed quality gates.

## Choose the review set

Before reviewing, present these options to the user and let them choose:

- **base** — the **Checklist** only: ID formats, US/FR/TC quality, and the six coverage
  rules. No analysis skills.
- **all** — `base` plus all seven analyses below.
- **subset** — `base` plus the analyses the user picks from the seven below.

The seven analyses:

| Analysis | Skill |
| --- | --- |
| failure-domain | `spec-failure-domain-analysis` |
| integrity | `spec-integrity-analysis` |
| dependency | `spec-dependency-analysis` |
| evidence | `spec-evidence-analysis` |
| risk-complexity | `spec-risk-complexity-analysis` |
| scope-boundary | `spec-scope-boundary-analysis` |
| ears-conformance | `spec-ears-analysis` |

## Process

1.  Automated Checks (always — part of `base`):
    -   Validate ID formats (US/FR/TC/AC/CON).
    -   Detect duplicates or gaps.
    -   Validation link integrity.
2.  Base Review (always):
    -   Read each artifact.
    -   Verify against the **Checklist**.
    -   Verify Test Coverage (6 Rules).
3.  Selected Analyses:
    -   Run each analysis the user chose. Prefer running them **in parallel**.
    -   **Fetch the template from quoin** once with `quoin write --types SpecReview` — use its
        skeleton + schema as the contract (do not invent the format).
    -   Write **one `SpecReview` document per analysis** to `spec/reviews/<analysis>.md`
        (plus `spec/reviews/base.md` for the base checklist). Findings go in a validated
        `## Findings` table (`| ID | Severity | Summary | Refs |`, `FND-NNN` ids, Severity ∈
        `low`/`medium`/`high`).
4.  Validate + record:
    -   Run `quire validate --scope <repo> "spec/**/*.md"` and fix any errors.

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

    -   The docs sync into filament-core as `SpecReview` artifacts; do not proceed to
        implementation until reviewed.

## Enforced runs (workflow mode)

`quoin review` drives this through `ix-flow`, which **records the chosen set and hard-blocks
acceptance until every selected analysis has produced a validated `SpecReview` doc** (the
`selected_analyses_covered` gate). Use it when you want the choice enforced rather than
trusted to discipline. The per-analysis `SpecReview` structure + the findings table are
defined in the workflow skill (`workflow-assets/skills/review/SKILL.md`).

## Common Issues

-   Vague Criteria: "Fast", "User-friendly". -> Make measurable.
-   Missing Errors: Only happy path. -> Add error modes.
-   No Coverage: AC without TC. -> Add tests.
-   Inconsistent IDs: `US-1` instead of `US-001`. -> Fix formatting.
