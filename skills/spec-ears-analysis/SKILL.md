---
name: spec-ears-analysis
description: Analyze requirement statements (FR/NFR/StR) for EARS requirement-grammar conformance and author a SpecReview of the findings.
---

# EARS Requirement-Grammar Analysis

This skill analyzes the natural-language **requirement statements** in a spec
(FR Description/Behavior/Constraints, NFR Statement, StR Stakeholder Need)
against **EARS** (Easy Approach to Requirements Syntax), and authors one
`SpecReview` document recording the findings.

It pairs the **deterministic engine check** (quire-rs FR-042, the `iso-spec-core`
grammar) with the **semantic judgment the engine cannot make** — together they
form the lens.

## Process

1.  **Run the engine check.** From the repo root:

    ```bash
    quire validate --scope . "spec/**/*.md" --summary
    ```

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


    Read the `[ears:<check>]` warnings and the `GrammarSummary` line. The engine
    flags, per statement:
    -   `non-singular` — more than one `shall` (can't map cleanly to one AC).
    -   `vague-response` — `support`/`handle`/`manage`/`process`/`provide`/
        `enable`/`be able to` (unverifiable).
    -   `missing-subject` — no named system/actor before `shall`.
    -   `non-canonical-trigger` — `On …`/`Upon`/`After`/`During` instead of
        `When …`/`While …`.
    -   `unclassifiable` — has `shall` but matches no EARS pattern.

    These are advisory (they never fail `quire validate`); this lens records
    them as review findings.

2.  **Add the semantic judgment** (what the engine cannot check deterministically):
    -   **Keyword-vs-intent** — does a `When …` statement describe a momentary
        *event*, or is it really a continuous `While …` *state* (or vice-versa)?
    -   **Genuinely concrete response** — a statement can pass the vague-verb
        lexicon yet still be unmeasurable ("shall be fast", "shall be robust").
    -   **Right pattern for the obligation** — an unwanted condition stated as
        `When …` instead of `If … then …`.

3.  **Author the SpecReview.** Fetch the template once:

    ```bash
    quoin write --types SpecReview
    ```

    Write one document to `spec/reviews/ears-conformance.md` with
    `analysis: ears-conformance` in the frontmatter, a `## Summary` (one or two
    sentences: how many statements, how many clean, the dominant defect), and a
    validated `## Findings` table:

    | ID | Severity | Summary | Refs |
    | --- | --- | --- | --- |
    | FND-001 | medium | FR-001 Description packs two `shall` into one statement — split into atomic requirements | FR-001 |
    | FND-002 | low | FR-003 leads with `On startup, …`; use `When the service starts, …` | FR-003 |

    -   `Severity` ∈ `low`/`medium`/`high`. Suggested mapping: `non-singular`
        and `missing-subject` → **medium** (they degrade traceability);
        `vague-response`/`unclassifiable` → **medium** when the response is truly
        unverifiable, else **low**; `non-canonical-trigger` → **low** (cosmetic).
        Raise to **high** any statement whose ambiguity changes what gets built.
    -   `Refs` names the offending FR/NFR/StR id.
    -   Combine the engine findings with your step-2 semantic findings into one
        table; attribute each row to its statement.

4.  **Validate + record.**

    ```bash
    quire validate --scope . "spec/reviews/ears-conformance.md"
    ```

    Fix any structural errors in the review doc. The doc syncs into filament-core
    as a `SpecReview` artifact like every other analysis lens.

## Notes

-   Scope is **requirement-bearing** artifacts only: FR, NFR, StR. US stories
    (`As a / I want / So that`) and tests are out of scope.
-   This lens is advisory by default. To make EARS a **gate** for a converted
    repo, run `quire validate --strict` in that repo's CI — `--strict` escalates
    the EARS warnings to a failing exit.
-   Per-archetype dialects the engine already applies (don't re-flag them): an
    enumerated `The X SHALL:` + numbered list is one statement; StR may use a
    stakeholder subject; an NFR with no trigger is fine.
