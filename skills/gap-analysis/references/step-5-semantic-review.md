# Step 5: Semantic Review (OPTIONAL)

**Goal**: Go beyond presence and traceability to *meaning* — for each requirement↔test↔code
triple, judge whether they actually agree. This is the expensive, judgment-heavy pass.

## Gate first

**Do not run this step unless the user opts in.** Ask explicitly:

> Run the optional semantic review (intent↔test↔code)? It's slower but verifies tests truly
> validate requirement intent and exercise real code, and that code matches intent.

If declined, skip and note "semantic review: skipped" in the SpecReview `## Coverage`.

## What to judge (per requirement)

For each requirement (FR/US/StR) in scope, with its tagged test(s) (from Step 3) and the
code it governs (from Step 4), assess three axes:

1. **(a) Test validates intent** — does the test actually check the *behavior the
   requirement describes*, including its acceptance criteria, or only an incidental/trivial
   aspect? A test tagged `FR-007-AC-1` that asserts something unrelated to AC-1 fails here.
2. **(b) Test exercises the code** — does the test run the real implementation, or is it
   hollow? Reuse Step 4's **test-stub / coverage-inflation** heuristics: no assertions,
   weak-only assertions (`is not None`, `isinstance`),
   mock-everything (no real code path), import-only, circular-mock-of-a-stub.
3. **(c) Code matches intent** — does the implementation actually do what the requirement
   says (semantics, edge cases, error behavior), or has it drifted?

Each failing axis → a finding:
- `high` — test does not validate intent **or** does not exercise code (false confidence),
  or code contradicts the requirement.
- `medium` — partial validation, meaningful edge cases unchecked, minor drift.

`Refs` = `FR-xxx` + test path + `path::symbol`.

## How to run it (thoroughness)

Fan out for breadth and independence — **one subagent per FR (or per cohesive area)**, each
returning a structured verdict per axis. Then aggregate the findings. This keeps each
judgment focused and lets many run concurrently. For a small scope, a single pass is fine.

## Output of this step

Per-requirement semantic findings (axis a/b/c failures) with `Refs`, plus a note for
`## Coverage` that semantic review ran and over how many requirements.

## Notes

- Keep verdicts evidence-based: quote the requirement clause, the assertion, and the code
  line that justify the finding. Avoid speculative "looks wrong" calls.
