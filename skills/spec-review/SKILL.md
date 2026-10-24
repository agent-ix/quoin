---
name: spec-review
description: Review requirements for quality, consistency, and completeness.
---

# Review Requirements

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

Use this skill to validate requirements artefacts before implementation.

## Checklist

See **[references/checklist.md](references/checklist.md)** for the detailed quality gates.

## Choose the review set

Inspect applicable active `AssuranceProfile` documents using the installed catalog.
The current contract is `review_policy.mode` (`recommend` or `require`) and
`review_policy.operations` (`spec-review`, `code-review`, `gap-analysis`).
Record the actual profile paths and an applicability disposition, or explicitly
state why none applies. Resolve ambiguous applicability with the owner.

A required `spec-review` operation needs an accepted enforced review run.
The profile does not select individual analysis lenses. Requirements for
`code-review` or `gap-analysis` remain separate operations; a specification
review cannot mark those complete. Never author or interpret `review_selection`.

For each review, present these options to the user and let them
choose:

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
    -   Flag every hash / digest / pin / tracking-record use as a `high` finding (see the
        **Hash / Digest / Pin Antipattern** section of the Checklist). The fix is removal;
        do not accept an existing use as a reason to keep it.
3.  Selected Analyses:
    -   Run each analysis the user chose. Prefer running them **in parallel**.
    -   **Fetch the template from quoin** once with `quoin write . --types SpecReview`
        from the target repository — use its skeleton + schema as the contract.
    -   Write **one `SpecReview` document per analysis** to `spec/reviews/<analysis>.md`
        (plus `spec/reviews/base.md` for the base checklist). Findings go in a validated
        `## Findings` table (`| ID | Severity | Summary | Refs |`, `FND-NNN` ids, Severity ∈
        `low`/`medium`/`high`).
4.  Validate + record:
    -   Run `quire validate --scope <repo> "spec/**/*.md"` and fix any errors.

> **`--scope` is the repository root, and must be passed explicitly.** The command derives **two roots** from it and never
> interchanges them: spec documents are read from `<repo>/spec` only, trace tags from
> the source tree at `<repo>` excluding `spec/`. A repo with no `spec/` exits with a
> diagnostic naming the missing document root rather than scanning the whole tree, and a
> matrix outside `spec/` (a fixture, a `plan/` copy) mints nothing. A relative glob
> resolves under `--scope` only in scoped mode (no `--module`); with `--module` it
> resolves against the process working directory, and an omitted `--scope` defaults to
> `.` — so a run launched from a parent directory validates the **wrong tree** and exits
> 0 for whatever it matched.

    -   The docs sync into filament-core as `SpecReview` artifacts; do not proceed to
        implementation until reviewed.

## Enforced runs (workflow mode)

`quoin review --target <repo-relative-path> --id <run-id> --json` starts the
Quoin-owned definition embedded in the native binary. Run it from the repository
root. Install published `@agent-ix/ix-flow` >=0.3.0; no private workflow package or
source checkout is needed. Quoin records repository/config roots and targets.

Use the materialized workflow's instructions and the same config/state roots for
all ix-flow commands. Record the `request` interview with `review_set`,
`selected_analyses`, `assurance_profiles`, and `profile_disposition`. Register
one `review_doc` item per analysis, including `base`, with its `id`, `analysis`,
and repository-relative `path`. The Rust provider opens and validates every
required document before validation and again before acceptance. Names or old
validation claims alone cannot pass. `all` always includes the seven lenses above.

The final transition requires explicit human acknowledgement; never automatically
acknowledge it. A failed enforced run cannot be claimed complete from direct
analyses. Direct review remains available when no applicable policy requires an
enforced spec-review operation. The gate verifies artifact validity, not reasoning
quality. Existing documents can be registered in a new owned run and revalidated;
old external-provider runs are not silently migrated or retrospectively accepted.

## Common Issues

-   Vague Criteria: "Fast", "User-friendly". -> Make measurable.
-   Missing Errors: Only happy path. -> Add error modes.
-   No Coverage: AC without TC. -> Add tests.
-   Inconsistent IDs: `US-1` instead of `US-001`. -> Fix formatting.

## Output contract

The `SpecReview` this skill emits carries `analysis: base` — the generic review value — this skill runs the analysis lenses and writes one document per lens, so it is the default rather than a lens of its own.

Stated here because nothing checked it (quoin#202): a skill and its declared
output were unlinked, so drift between them was invisible until a document
failed validation in an unrelated session.
