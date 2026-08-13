# Step 4: Generate Tests

**Goal**: write the test, tagged so `gap-analysis` can reconcile it.

Templates per harness:
[proptest](../assets/templates/proptest.rs.md) ·
[fast-check](../assets/templates/fast-check.ts.md) ·
[hypothesis](../assets/templates/hypothesis.py.md)

## The tag contract

`gap-analysis` step 3 greps for four forms. Every generated test carries **two independent
carriers**, so a single formatting change cannot break reconciliation:

1. `Trace: FR-027-AC-1` on its own line in the doc comment or docstring.
2. The test function or `describe` name containing the id —
   `fr_027_ac_1_…` in Rust/Python, `"FR-027-AC-1 …"` in TypeScript.

### Where carrier 1 attaches

A carrier is not free-floating text. `quire coverage` binds a tag to the **test symbol
whose source span encloses it** — the span covering that symbol's annotation block, its
declaration, and its body. Placement is therefore part of the contract, and it differs by
language:

| Harness | Put `Trace:` | Because |
|---------|--------------|---------|
| Rust | in the doc comment above `#[test] fn …` | the span starts at the annotation block |
| Python | in the test function's docstring | the span covers the body |
| TypeScript | **immediately above `it(` / `test(`** — never above `describe(` | `describe(…)` groups tests but registers no symbol itself |

The TypeScript row is the one that bites. A tag above a `describe` block reads correctly,
passes review, and matches a grep — and binds to nothing at all. Six generated files
shipped that way in `@agent-ix/quoin@0.12.x` and every criterion in them scored zero
(agent-ix/quoin#61). **Do not verify placement with grep**: grep does not care where a
comment sits, which is exactly why the defect shipped.

Plus one provenance line, ignored by the reconciliation grep and read by this skill on
re-runs:

```
spec-correctness: row=FR-027-AC-1 property=universal extraction=extractable origin=regex
```

`origin ∈ {regex, regex-candidate, llm-second-pass}`.
For `origin=llm-second-pass`, add `confidence=<high|medium|low>`.

Hard rules:

- **Never emit a `row_id` that is not in the classification output.** Not a guessed one,
  not a renumbered one, not one you inferred from a nearby row.
- **Never alter an existing tag** on a hand-written test. If a test already claims the
  `row_id`, do not emit a second one — record it as already covered.
- Match the repo's dominant tag style from step 0 **in addition to** the two carriers.

## Idempotent re-runs

The provenance line is the key. On a re-run:

- Same `row_id`, same strategy, same grounding → leave the file alone.
- Same `row_id`, changed grounding → rewrite the body, keep the tag byte-identical.
- `row_id` no longer in the classification output (the criterion was deleted) → do not
  delete the test silently; report it as orphaned and let the user decide.
- A test whose provenance line is absent is hand-written. **Never overwrite it.**

## File placement

| Harness | Path |
| --- | --- |
| Rust | `tests/props_fr_NNN.rs` |
| TypeScript | `tests/props/fr-NNN.prop.test.ts` |
| Python | `tests/props/test_fr_NNN.py` |

One placement, because there is one kind of emitted test. A criterion that cannot be
grounded gets **no file at all** — it gets a finding in the review artifact (step 6).

**Rust paths are flat on purpose.** Cargo auto-discovers integration tests only at
`tests/*.rs`; a file under `tests/props/` is treated as a helper module and is never
compiled as a test target. A nested layout would need a `[[test]]` entry per file in
`Cargo.toml`, and this skill does not edit build manifests. Flat names with a `props_`
prefix keep the grouping without the manifest edit.

For a Rust repo that keeps `#[cfg(test)]` unit tests inline and no `tests/` dir at all,
still write `tests/props_fr_NNN.rs` — an integration test only sees the public API, which is
the right boundary for a criterion-derived property anyway. If the symbol under test is
private, that is a grounding result, not a placement problem: write no test and record
`symbol-not-public` as a finding.

One file per FR, one test per `row_id`. Shared generators go in a sibling `arbitraries`
module rather than being duplicated per test.

## Every emitted test runs

No skip marker, no `#[ignore]`, no `_review/` directory. A disabled test checked into a
repo is a dead test, and the review it was waiting for already happens in the pull request
the test arrives in.

Whether a matrix row reads `✅` is decided by a real run and set by `spec-matrix` — not
pre-empted here with a marker. A criterion that is not ready to be tested is not a disabled
test; it is a finding (step 6).

## Generator hygiene

- Bound every collection (`0..32` is a good default). An unbounded generator turns a fast
  suite slow and hides shrinking.
- Reuse the repo's existing fixtures and factories before writing a new arbitrary.
- Seed generators so both branches of a partitioned domain are reachable — check the
  sibling ACs from step 2.
- Do not set a fixed RNG seed. Do not raise the case count above the harness default
  without a reason recorded as a finding.

## After writing

Run the suite. A failing generated test is a grounding bug in step 2, not a spec bug
and not a code bug — fix the grounding, or delete the test and record the reason as a
finding. Never weaken an assertion to make a generated test pass, never disable it, and
never edit the spec to match a generated test.
