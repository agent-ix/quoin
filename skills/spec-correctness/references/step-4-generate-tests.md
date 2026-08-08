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

Plus one provenance line, ignored by the reconciliation grep and read by this skill on
re-runs:

```
spec-correctness: row=FR-027-AC-1 property=universal extraction=extractable origin=regex review=none
```

`origin ∈ {regex, regex-candidate, llm-second-pass}`. `review ∈ {none, required}`.
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

| Harness | Unattended | Queued |
| --- | --- | --- |
| Rust | `tests/props_fr_NNN.rs` | `tests/props_review_fr_NNN.rs` |
| TypeScript | `tests/props/fr-NNN.prop.test.ts` | `tests/props/_review/fr-NNN.prop.test.ts` |
| Python | `tests/props/test_fr_NNN.py` | `tests/props/_review/test_fr_NNN.py` |

**Rust paths are flat on purpose.** Cargo auto-discovers integration tests only at
`tests/*.rs`; a file under `tests/props/` is treated as a helper module and is never
compiled as a test target. A nested layout would need a `[[test]]` entry per file in
`Cargo.toml`, and this skill does not edit build manifests. Flat names with a `props_`
prefix keep the grouping without the manifest edit.

For a Rust repo that keeps `#[cfg(test)]` unit tests inline and no `tests/` dir at all,
still write `tests/props_fr_NNN.rs` — an integration test only sees the public API, which is
the right boundary for a criterion-derived property anyway. If the symbol under test is
private, that is a grounding result, not a placement problem: record `symbol-not-public` and
queue it.

One file per FR, one test per `row_id`. Shared generators go in a sibling `arbitraries`
module rather than being duplicated per test.

## Inert markers for queued tests

A queued test must not be able to turn a matrix row green:

| Harness | Marker |
| --- | --- |
| Rust | `#[ignore = "spec-correctness review pending"]` |
| TypeScript | `it.skip(…)` / `describe.skip(…)` |
| Python | `@pytest.mark.skip(reason="spec-correctness review pending")` |

The tag stays byte-identical whether the test is queued or accepted — so
`gap-analysis` reports the same identifier before and after, and acceptance is a
one-line diff plus a move.

## Generator hygiene

- Bound every collection (`0..32` is a good default). An unbounded generator turns a fast
  suite slow and hides shrinking.
- Reuse the repo's existing fixtures and factories before writing a new arbitrary.
- Seed generators so both branches of a partitioned domain are reachable — check the
  sibling ACs from step 2.
- Do not set a fixed RNG seed. Do not raise the case count above the harness default
  without a reason recorded in the queue.

## After writing

Run the suite. A failing **unattended** test is a grounding bug in step 2, not a spec bug
and not a code bug — fix the grounding or move the test to the queue with a reason. Never
weaken an assertion to make a generated test pass, and never edit the spec to match a
generated test.
