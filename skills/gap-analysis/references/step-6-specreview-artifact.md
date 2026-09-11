# SpecReview Artifact

**Goal**: Emit the findings from the preceding steps as a single **quire-validated
`SpecReview`** document with a Verdict, at `reviews/YY-MM-DD-<slug>.md`.

The artifact must make the mode unambiguous. A reader who sees only this document has to be
able to tell whether plan completion was assessed or skipped, and must not be able to read a
planless PASS as evidence that planned work was completed.

## Render the template, then author (guardrail)

The `SpecReview` archetype lives in `spec-artifacts-process`. Render its skeleton first,
then author, then validate (the ecosystem direct-render-then-validate model):

```
quoin write --types SpecReview
```

If `quoin write` is unavailable, use the complete frontmatter, body, findings-table, and
validation contract below. Do not consult a private repository, user-home path, or
separately installed template.

## Frontmatter

**Planless (the default).** No plan id appears anywhere in the frontmatter:

```yaml
---
id: SR-001                      # ^[A-Z]{2,4}-[0-9]+$ — SR- default is fine; bump if SR-001 exists
title: "Gap analysis — <component> repository audit"
type: SpecReview
analysis: gap-analysis          # the dedicated analysis value
scope: "spec/, src/, tests/, spec/matrix.md"
review_set: subset
relationships:
  - { target: "ix://<org>/<component>/<TestMatrix-id>", type: references }
---
```

**Plan-assisted.** Identical, plus the `reviews` edge to the supplied plan:

```yaml
---
id: SR-001
title: "Gap analysis — <component> repository audit (with <Plan-id>)"
type: SpecReview
analysis: gap-analysis
scope: "spec/, src/, tests/, spec/matrix.md, plan/<Plan-id>-<slug>/"
review_set: subset
relationships:
  - { target: "ix://<org>/<component>/<Plan-id>",       type: reviews }
  - { target: "ix://<org>/<component>/<TestMatrix-id>", type: references }
---
```

`<org>`/`<component>` come from `spec/spec.md` (`org`, `name`); the `<TestMatrix-id>` and
the optional `<Plan-id>` from target selection. Never emit a `reviews` edge to a plan in a
planless run — a relationship to a plan nobody audited is a fabricated claim.

## Body

`## Summary` and `## Findings` are **required and validated** by `quire validate`; the others
are extra sections (allowed).

```markdown
## Summary

<1–2 sentences: which repository's spec, matrix, tests and code were audited, and the
headline result. In planless mode, say the audit was repository-driven and that plan
completion was not assessed. Do not describe the result as work being "complete" or
"delivered as planned".>

## Verdict

**PASS | CONDITIONAL | FAIL** — <one line justifying the gate, per the verdict rule>.

## Findings

| ID      | Severity | Summary                                          | Refs               |
| ------- | -------- | ------------------------------------------------ | ------------------ |
| FND-001 | high     | Matrix TC-012 has no backing tagged test         | TC-012, FR-006     |
| FND-002 | medium   | `cli.ts::--force` flag has no owning requirement | cli.ts::--force    |
| FND-003 | high     | Task-007 still in_progress (P0) — plan-assisted  | Task-007, FR-004   |

## Coverage

- Reconciliation: quire coverage (module <name> <version>) | grep fallback — no active module declares a traceability model
- Plan completion: not assessed
- Rows backed by a tagged test: X / Y   (from `totals`; `0 / 0` means the model matched nothing, not full coverage)
- Untraced behaviors / stubs: N
- Semantic review: ran over N requirements | skipped
```

### The plan-completion line (mandatory, both modes)

`## Coverage` always carries exactly one plan-completion line, and it is not optional:

| Mode | Line |
| --- | --- |
| Planless | `Plan completion: not assessed` |
| Plan-assisted | `Plan completion: assessed (<Plan-id>) — tasks done X / Y` |

`Plan completion: not assessed` is the literal wording. Do not soften it to "n/a", "none",
or "no plan required", and never write `Tasks done: 0 / 0` in a planless run — a zero
denominator reads as a completed plan and asserts something nobody checked.

The reconciliation line is not optional. A number from the engine and a number from a grep
are not the same claim — grep matches a tag wherever it sits, including places the engine
will not bind it — and a reader cannot tell which they are looking at unless it says so.

### Findings table contract (validated)

- Headers EXACTLY: `ID | Severity | Summary | Refs`.
- `ID` matches `^FND-\d+$`; ≥1 row.
- `Severity` ∈ `low | medium | high`.
- A clean audit still records one row: `FND-001 | low | No gaps found | -`.

## Verdict rule

- **FAIL** — any unbacked matrix Test Case, any `high` finding, or (plan-assisted only) any
  incomplete/blocked task.
- **CONDITIONAL** — only `medium`/`low` findings.
- **PASS** — no gaps (single `No gaps found` row).

### What a planless PASS means

A planless PASS asserts **repository assurance only**: the matrix rows are backed by tagged
tests, no meaningful code lacks an owning requirement, and no stub or inflated coverage
stands behind a claim. It asserts nothing about whether the work was planned, tracked, or
completed against a plan — that question was not asked. Keep the Summary and Verdict lines
consistent with that, and leave `Plan completion: not assessed` visible in `## Coverage` as
the record of what the PASS does not cover.

## Validate

```
quire validate --scope <project_root> "reviews/**/*.md"
```

**Always pass `--scope <project_root>` explicitly, as above.** A relative glob resolves
under `--scope` only in scoped mode (no `--module`); with `--module` it resolves against
the process working directory, and an omitted `--scope` defaults to `.`. A sweep launched
from a parent directory therefore validates the **wrong tree** and exits 0 for whatever
it did match — that is the trap. (A glob matching *nothing* is not silent: the CLI exits
1 with `document glob matched no files`. Only the wrong-tree half is quiet.)

Fix any validation error (frontmatter pattern, `analysis` enum, findings headers/ids/severity)
before reporting completion. Then tell the user the artifact path and the Verdict.

## Notes

- File name: `reviews/<YYYY-MM-DD>-<short-slug>.md` (today's real date; slug from the
  component or area audited, e.g. `2026-06-22-observation-gap-analysis`, or from the plan in
  plan-assisted mode, e.g. `2026-06-22-plan-002-packaging`).
- `reviews/` is **repo root** for this skill (deliberate); validation is path-agnostic.
- One run → one SpecReview doc (the `analysis: gap-analysis` lens), matching the one-doc-per-
  analysis model used by `quoin:spec-review`.
- This document is the run's **only** write. Do not also create a plan, a `Task`, a
  requirement, or an acceptance criterion to make the findings actionable — filing that work
  is a separate, later decision.
