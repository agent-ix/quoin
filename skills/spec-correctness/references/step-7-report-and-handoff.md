# Step 7: Report and Handoff

**Goal**: close the loop — the matrix knows about the new tests, and `gap-analysis` can
reconcile every tag this run emitted.

## The run report

Prepended to `tests/props/QUEUE.md`. Counts only:

```
spec-correctness — <repo> — <YYYY-MM-DD>
quire-cli <version> · harness <name> · <N> criteria

emitted unattended   37   (extractable, grounded)
queued               22   candidate 0 · second-pass 18 · downgraded 1 · dep-missing 3
refused              14   symbol-not-found 6 · oracle-is-adjectival 5 · unimplemented 3
already covered       9   hand-written tests already carry the row_id
witnesses             5   Unit tests, not property coverage
```

`emitted unattended + queued + refused + already covered` must equal the number of records
with a `row_id`. If it does not, a record was dropped — find it before reporting.

No thresholds, no grades, no rewording suggestions. Same rule as step 1.

## Handoff to `spec-matrix`

Emit rows for the `spec-matrix` Test Case Summary — do not write them yourself if the repo
already runs the matrix workflow; hand them over.

`Test ID | Title | Type | Priority | Traces To | Status`

- `Type` — `Property` for the 8 generatable families; `Unit` for witnesses, whether they
  came from the second pass or from a `singleton-domain` grounding result.
  The vocabulary is owned by `spec_artifacts_process/manifest.yaml`, not by this skill; if
  it disagrees with what you write, the manifest wins.
- `Traces To` — the `row_id`, exactly (`FR-027-AC-1`). Never a range you invented, never
  another TC.
- `Status` — `✅` only for an unattended test that passes. `🚧` for anything queued.
  Refused criteria get **no row**; they are reported in the queue, not claimed in the
  matrix.

## Binding check

Before finishing, prove every emitted tag actually **binds** — not that it exists.

```
quire coverage --scope <repo> --json
```

For each `row_id` this run emitted, confirm the id appears among the backed ids: its
minting document's group must count it, and it must not appear in `untracked_symbols`.

```
quire coverage --scope <repo> --json \
  | jq -r '.untracked_symbols[] | "\(.trace_id)\t\(.path)\t\(.symbol)"'
```

An emitted `row_id` that is *not* backed means the tag is in the file but attached to no
test symbol — almost always TypeScript placement, a tag above `describe(` instead of above
`it(` (step 4). Fix the placement and re-run; do not report the criterion as covered.

**This check replaces a grep, deliberately.** The previous version of this step confirmed
the tags were greppable, and a grep matches a comment wherever it sits. Six generated files
passed that check with every tag bound to nothing (agent-ix/quoin#61). Only the engine that
consumes the tags can tell you a tag works.

Then confirm, as before:

- no matrix row this run added is `✅` while its test is skipped;
- no emitted tag names a `row_id` absent from the `quire properties` output.

If `quire coverage` reports `no rows matched`, the module in scope declares no traceability
model for this repo's layout — that is an environment gap, not a clean run. Say so rather
than reporting the run as reconciled.

A mismatch here is a bug in this run, not in `gap-analysis`.

## Finish by saying

- What now has tests, in concrete terms: which FRs, how many criteria.
- What is waiting in the queue and roughly how long a review pass would take.
- The one next action — usually "review `tests/props/QUEUE.md`" or "add `<lib>` to
  dev-dependencies so the queued tests can run".
