# Step 6: SpecReview Artifact

**Goal**: Emit the findings from Steps 1–5 as a single **quire-validated `SpecReview`**
document with a Verdict, at `reviews/YY-MM-DD-<slug>.md`.

## Fetch the template, then author (guardrail)

The `SpecReview` archetype lives in `spec-artifacts-process`. Fetch its skeleton first, then
author, then validate (the ecosystem direct-render-then-validate model):

```
quoin write --types SpecReview
```

If `quoin write` is unavailable in the working tree, read the skeleton directly from
`~/.ix/filament/modules/spec-artifacts-process/skeletons/SpecReview.md`.

## Frontmatter

```yaml
---
id: SR-001                      # ^[A-Z]{2,4}-[0-9]+$ — SR- default is fine; bump if SR-001 exists
title: "Gap analysis — <Plan-id> <slug>"
type: SpecReview
analysis: gap-analysis          # the dedicated analysis value
scope: "plan/<Plan-id>-<slug>/, spec/matrix.md"
review_set: subset
relationships:
  - { target: "ix://<org>/<component>/<Plan-id>",       type: reviews }
  - { target: "ix://<org>/<component>/<TestMatrix-id>", type: references }
---
```

`<org>`/`<component>` come from `spec/spec.md` (`org`, `name`); the `<Plan-id>` and
`<TestMatrix-id>` from Step 1.

## Body

`## Summary` and `## Findings` are **required and validated** by `quire validate`; the others
are extra sections (allowed).

```markdown
## Summary

<1–2 sentences: what plan/matrix/code was audited and the headline result.>

## Verdict

**PASS | CONDITIONAL | FAIL** — <one line justifying the gate, per the verdict rule>.

## Findings

| ID      | Severity | Summary                                          | Refs               |
| ------- | -------- | ------------------------------------------------ | ------------------ |
| FND-001 | high     | Task-007 still in_progress (P0, critical path)   | Task-007, FR-004   |
| FND-002 | high     | Matrix TC-012 has no backing tagged test         | TC-012, FR-006     |
| FND-003 | medium   | `cli.ts::--force` flag has no owning requirement | cli.ts::--force    |

## Coverage

- Reconciliation: quire coverage (module <name> <version>) | grep fallback — no active module declares a traceability model
- Tasks done: X / Y
- Rows backed by a tagged test: X / Y   (from `totals`; `0 / 0` means the model matched nothing, not full coverage)
- Untraced behaviors / stubs: N
- Semantic review: ran over N requirements | skipped
```

The reconciliation line is not optional. A number from the engine and a number from a grep
are not the same claim — grep matches a tag wherever it sits, including places the engine
will not bind it — and a reader cannot tell which they are looking at unless it says so.

### Findings table contract (validated)

- Headers EXACTLY: `ID | Severity | Summary | Refs`.
- `ID` matches `^FND-\d+$`; ≥1 row.
- `Severity` ∈ `low | medium | high`.
- A clean audit still records one row: `FND-001 | low | No gaps found | -`.

## Verdict rule

- **FAIL** — any incomplete/blocked task, any unbacked matrix Test Case, or any `high` finding.
- **CONDITIONAL** — only `medium`/`low` findings.
- **PASS** — no gaps (single `No gaps found` row).

## Validate

```
quire validate --scope <project_root> "reviews/**/*.md"
```

**Always pass `--scope <project_root>` explicitly, as above.** A relative glob resolves
under `--scope` only in scoped mode (no `--module`); with `--module` it resolves against
the process working directory, and an omitted `--scope` defaults to `.` — either way a
sweep launched from a parent directory silently validates the wrong tree (or nothing)
while exiting 0 for whatever it did match.

Fix any validation error (frontmatter pattern, `analysis` enum, findings headers/ids/severity)
before reporting completion. Then tell the user the artifact path and the Verdict.

## Notes

- File name: `reviews/<YYYY-MM-DD>-<short-slug>.md` (today's real date; slug from the plan,
  e.g. `2026-06-22-plan-002-packaging`).
- `reviews/` is **repo root** for this skill (deliberate); validation is path-agnostic.
- One run → one SpecReview doc (the `analysis: gap-analysis` lens), matching the one-doc-per-
  analysis model used by `quoin:spec-review`.
