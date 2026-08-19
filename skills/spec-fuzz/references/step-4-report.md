# Step 4: Report

**Goal**: a validated artifact saying what was generated, what was not, and what none of it
proves yet.

## The handoff sentence that must appear

> These targets are **undischarged**. A fuzz target that has never run is a search never
> started, and the evidence store will report it as such until a run is recorded.

Say it plainly and do not soften it. The temptation at the end of a generation run is to
report N targets as N obligations covered, and that is the exact claim this skill exists not
to make (FR-038-CON-3).

What the consumer does next is theirs: run the fuzzer in their CI, then
`quoin evidence record` the result into a suite with `Evidence Kind: Fuzz`. quoin does not
run it — the invariant that makes a run record mean *"this ran in your CI"* is that quoin was
not the thing running it.

## The artifact

A `SpecReview` with `analysis: evidence`, at `reviews/YY-MM-DD-<slug>.md`. One findings row
per obligation **not served**, and none for the ones that were — a review listing every
success is a review nobody reads to the end.

```markdown
| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | This repository has no fuzz harness; 6 Fuzz-kind obligations cannot be materialized. Nothing was installed. | Cargo.toml |
| FND-002 | medium | NFR-003-M-1 names no groundable entry point: no function in `src/` takes arbitrary bytes. | NFR-003-M-1 |
| FND-003 | low | FR-012-AC-2 is authored `fuzzing` but describes an ordering, not an input surface. The method looks wrong. | FR-012-AC-2 |
```

Severities, and why:

- **high** — no harness. Every Fuzz-kind obligation in the repository is unmaterializable, and
  the decision to change that is the owner's.
- **medium** — ungrounded entry point. A gap in the spec or the code, one obligation each.
- **low** — a method that looks wrong for its obligation. Advisory; the author decides.

## Report counts, not a grade

*"6 of 9 obligations produced targets"* is data. **"67% fuzz coverage" is a verdict**, and
this skill emits none — no threshold, no grade, no pass/fail (FR-038-AC-9).

The same rule bars rewording. Where a requirement is genuinely unfuzzable as stated, that is
a finding for a human, not an edit for this skill to make.

## Validate before finishing

```
quire validate --scope <repo> "reviews/**/*.md"
```

Fix any frontmatter, `analysis` enum or findings-table error before reporting the run
complete. A review that does not validate is not an artifact; it is a file.
