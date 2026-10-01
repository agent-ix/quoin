# Step 7: Report and Handoff

**Goal**: close the loop — every emitted tag binds, so the computed matrix (`quire matrix`)
shows each covered criterion as `tagged` and `gap-analysis` reads the same result.

## The run report

The `## Census` section of the review artifact (step 6), and the same counts to the user.
Counts only:

```
spec-correctness — <repo> — <YYYY-MM-DD>
quire-cli <version> · harness <name> · <N> criteria

emitted              37   extractable, grounded, no finding
emitted + finding    22   candidate 0 · second-pass 18 · downgraded 1 · dep-missing 3
not settled          14   symbol-not-found 6 · oracle-is-adjectival 5 · unimplemented 3
already covered       9   hand-written tests already carry the row_id
witnesses             5   Unit tests, not property coverage
```

`emitted + emitted-with-finding + not-settled + already-covered` must equal the number of
records with a `row_id`. If it does not, a record was dropped — find it before reporting.

No thresholds, no grades, no rewording suggestions. Same rule as step 1.

## No matrix to write

The Test Matrix is computed from the criteria and the tags this run emitted. Write no
matrix rows, no `tests.md` and no `TC-` ids. A criterion with no test is a finding in the
review artifact, and `quire matrix` shows it as `untagged`.

## Binding check

Before finishing, prove every emitted tag actually **binds** — not that it exists.

```
quire coverage --scope <repo> --json
```

`--scope` is the repository root. The command
derives two roots from it: documents from `<repo>/spec` only, trace tags from the source
tree excluding `spec/`. A repo with no `spec/` directory exits with a diagnostic naming
the missing document root, and a document outside `spec/` mints nothing.

For each `row_id` this run emitted, confirm `quire matrix --scope <repo> --format json`
reports the criterion `tagged` with this run's test among its `binders`, and that the id
does not appear in `untracked_symbols`.

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

- no criterion this run tagged reads `tagged-by-ignored-test`;
- no emitted tag names a `row_id` absent from the `quire properties` output.

Two distinct states get conflated here; keep them apart (they have different causes and
different fixes — `gap-analysis` step 3 has the same reading):

- **A zero denominator.** Under the `--json` invocation this step prescribes the signal is
  `"totals": {"total": 0}`. `no rows matched` is a **human-format-only** marker and never
  appears in the JSON, so do not look for it there. It means the declared model matched
  nothing in this scope — no minting document was found. Not full coverage, not a clean
  run: report it as **no data**.
- **No model at all.** `quire coverage` exits **non-zero** with a distinct diagnostic when
  no active module declares a `traceability:` model (quire-rs FR-050-AC-9). That is an
  environment gap — the module set, not the repo's rows.

Say which one happened rather than reporting the run as reconciled.

A mismatch here is a bug in this run, not in `gap-analysis`.

## Finish by saying

- What now has tests, in concrete terms: which FRs, how many criteria.
- How many findings are waiting and roughly how long reading them would take.
- The one next action — usually "read `reviews/<date>-<slug>.md`" or "add `<lib>` to
  dev-dependencies, then re-run to emit the tests it blocked".
