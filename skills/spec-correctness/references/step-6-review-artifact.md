# Step 6: The Review Artifact

**Goal**: the run leaves a record of **what it could not ground, and why**, as a validated
artifact a reviewer can read without re-deriving anything.

The tests it *could* write are already under review — they arrive in a pull request, which
is what a review gate is. What a PR does not carry is the reasoning behind a criterion the
run declined to test. That, and only that, is what this artifact is for.

## Where it lives

`reviews/YY-MM-DD-<slug>.md` at the repo root — the same location and archetype
`gap-analysis` uses. It is a **`SpecReview`** with `analysis: spec-correctness`.

> Earlier versions of this skill wrote `tests/props/QUEUE.md` instead, because
> `SpecReview.analysis` had no `spec-correctness` value and a file under `reviews/` would
> have failed `quire validate`. A closed enum with no fitting value is the system saying an
> artifact type is missing; it was read as an obstacle and routed around, and what it
> produced was an unvalidated report committed into a test tree. The enum was fixed in
> agent-ix/spec-artifacts-process#11. **Do not invent an output format** (FR-028-CON-1) —
> if nothing fits, that is a ticket, not a new filename.

## Fetch the template, then author

```
quoin write --types SpecReview
```

If `quoin write` is unavailable in the working tree, read the skeleton directly from
`~/.ix/filament/modules/spec-artifacts-process/skeletons/SpecReview.md`.

## Frontmatter

```yaml
---
id: SR-002                       # ^[A-Z]{2,4}-[0-9]+$ — bump past any existing SR-
title: "spec-correctness — <repo> — <N> criteria"
type: SpecReview
analysis: spec-correctness       # the dedicated analysis value
scope: "spec/**/*.md, tests/props/"
review_set: all
---
```

## Body

`## Summary` and `## Findings` are **required and validated**; the rest are extra sections.

```markdown
## Summary

<1–2 sentences: what corpus was classified, how many criteria, and how many are now
covered by a generated test.>

## Findings

| ID      | Severity | Summary                                                       | Refs         |
| ------- | -------- | ------------------------------------------------------------- | ------------ |
| FND-001 | medium   | Candidate label not corroborated structurally — verify the shape | FR-018-AC-3  |
| FND-002 | medium   | Oracle is adjectival ("clear error"); nothing to assert         | FR-022-AC-1  |
| FND-003 | low      | Domain is a single value; a Unit witness, not property coverage | FR-009-AC-2  |
| FND-004 | high     | No generator library in the manifest; no test written           | FR-031-AC-1  |

## Census

emitted        37   extractable, grounded
reviewed        6   emitted and flagged (`extraction: candidate`)
not settled    14   symbol-not-found 6 · oracle-is-adjectival 5 · unimplemented 3
already covered 9   hand-written tests already carry the row_id
witnesses       5   Unit tests, not property coverage
```

### Findings table contract (validated)

- Headers EXACTLY: `ID | Severity | Summary | Refs`.
- `ID` matches `^FND-\d+$`; **≥1 row**.
- `Severity` ∈ `low | medium | high`.
- A run that grounded everything still records one row:
  `FND-001 | low | Every binding criterion was grounded | -`.

## What each finding is

| Reason | Severity | Note |
| --- | --- | --- |
| `extraction: candidate` | medium | A test **was** emitted; the metamorphic label was not corroborated structurally, so the reviewer checks the shape |
| Second-pass reclassify | medium | An LLM read, not a deterministic one |
| Second-pass witness | low | An example test, not property coverage |
| `singleton-domain` witness | low | Grounding found a one-element domain — a `Unit` test |
| Harness-downgraded | low | e.g. Python `concurrency` |
| Not grounded | medium | No test written; the reason **is** the finding |
| Generator library missing | high | No test written; name the library and the remedy |

## Severity is about the reader's next action, not about quality

`high` means a person must do something before this criterion can be covered at all —
install a library, implement a symbol. `medium` means read the generated test and confirm
it. `low` means know that a row is a witness rather than a property. None of it is a
verdict on the spec (FR-052-CON-1), and none of it recommends rewording a criterion.

## Emitted tests are not disabled

Every test this skill writes **runs**. No `describe.skip`, no `#[ignore]`, no
`@pytest.mark.skip`, no `_review/` directory. A disabled test in a repo is a dead test, and
whether a matrix row reads `✅` is the `spec-matrix` skill's status column to set from a
real run — not this skill's to pre-empt with a marker.

## Validate

```
quire validate --scope <project_root> "reviews/**/*.md"
```

Fix any validation error — frontmatter pattern, the `analysis` enum, findings
headers/ids/severity — before reporting completion. Then tell the user the artifact path.

## Notes

- File name: `reviews/<YYYY-MM-DD>-<short-slug>.md`, today's real date.
- One run → one `SpecReview`, matching the one-doc-per-analysis model.
- A re-run reads the previous artifact's findings so it does not re-propose a criterion a
  reviewer already rejected; a criterion whose `statement` has changed is proposed again.
