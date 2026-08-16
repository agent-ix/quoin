# Step 3: Test-Matrix Verification

**Goal**: Prove the Test Matrix is *real* — every Test Case it claims is backed by an actual
test in the suite, identified by a matching **tracking tag** in the test code. This is the
heart of gap-analysis: a matrix row marked ✅ means nothing unless a tagged test exists.

**This step no longer greps.** `quire coverage` computes the reconciliation deterministically
and reports it; this skill interprets the report and owns the judgement. Severity and verdict
stay here — the command reports and does not judge (quire-rs FR-050-CON-1).

## The matrix shape

Built by `spec-matrix` (`quoin/skills/spec-matrix/SKILL.md`). Key tables:

- **Test Case Summary** — `Test ID | Title | Type | Priority | Traces To | Status`, where
  `Test ID` = `TC-xxx`, `Traces To` = `FR-XXX-AC-X` etc., `Status` ∈ `✅ ⚠️ ❌ 🚧 ⛔`.
- **Per-requirement coverage** — StR/US/FR/NFR → AC → TC → Status tables.

What each column *means* is not this skill's knowledge either: the active module declares it
under `traceability:`, and `quire coverage` reads the same declaration the matrix contract is
validated against, so the two cannot drift.

## Run the rollup

```
quire coverage --scope <project_root> --json
```

`--scope` is the repository root, and stays so. Since quire-cli v0.16.0 (quire-rs CR-045)
the command derives **two roots** from it and never interchanges them: spec documents are
read from `<project_root>/spec` only, and trace tags from the source tree at
`<project_root>` excluding `spec/`. This invocation needs no second flag — but a project
whose `spec/` directory is missing now exits with a diagnostic naming the missing document
root instead of silently scanning the whole repository, and a matrix outside `spec/` (a
fixture, a `plan/` copy) mints nothing.

**Check the version before relying on any of that.** The "since v0.16.0" premise is not
enforced anywhere — nothing probes it, and a user on ≤ 0.15.0 silently gets the pre-split
traversal semantics, where the walk covers the whole repository and a matrix outside
`spec/` mints. One line, before the first invocation:

```bash
quire --version    # expect >= 0.16.0; on an older build the roots are not split
```

If it is older, say so in `## Coverage` rather than reading the report as if the split
applied.

Do **not** pass `--strict`. Whether a gap blocks is this skill's verdict rule (Step 6), not
the command's exit code.

The report carries exactly the findings this step produces:

| Report field | What it is |
| --- | --- |
| `unbacked_rows` | A declared reference row whose trace targets have no backing `verifies` relation. Each carries `reference`, `document`, `row_id`, `target_ids`. |
| `status_lies` | A row whose status classes as `complete` while nothing backs it. Adds the authored `status` string. |
| `untracked_symbols` | A test carrying a trace tag that resolves to no declared row. Carries `path`, `symbol`, `trace_id`. |
| `no_symbol_rows` | An unbacked row whose **declared verification method mints no source symbol** — an eval, an inspection, a demonstration (quire-rs FR-050-AC-16 / CR-041). Carries `reference`, `document`, `row_id`, `test_type`, `target_ids`. **Read this before triaging `unbacked_rows`**: a row listed here is exempt from `status_lies` by its own method, and reporting it as an unbacked-row finding asserts something impossible. Absent from the JSON entirely when the module declares no `no_source_symbol` vocabulary. |
| `criteria` | Per-document acceptance-criteria property counts (quire-rs FR-050-AC-13 / CR-028): `document`, `archetype`, `criteria`, `property_shaped`, `by_property`. Data for the `spec-correctness` handoff, never a verdict — a low property-shaped share describes a corpus, it does not fail one. Absent when the corpus binds no criteria. |
| `groups` | Per minting document: `document`, `target`, `backed`, `total`. |
| `diagnostics` | Declarations that selected nothing and why (quire-rs FR-050-AC-19 / CR-054): an unreadable declared `document:`, an archetype no document has when the model minted nothing, or a model with no trace targets. Absent when every declaration selected. A non-empty list means the numbers below it are measuring less than you think. |
| `totals` | `backed` / `total` across the bundle, plus `criteria` / `property_shaped` when the corpus binds criteria. |

## Reconcile, producing findings

| Report field | Finding | Severity |
| --- | --- | --- |
| `unbacked_rows` | Matrix overclaims coverage — the row names a criterion or test id nothing backs | `high` |
| `status_lies` | The row asserts `✅` over nothing. A subset of the above, and the worse half | `high` |
| `untracked_symbols` | A tagged test pointing at a row that does not exist — a stale tag, or a matrix that dropped a row | `medium` |
| Marker drift | Matrix `Status` inconsistent with a real run (`🚧` on a passing tagged test) | `low` |

`Refs` for each finding is the `row_id` and `document` the report gives, or `path::symbol`
for an untracked symbol. Do not re-derive them.

Marker drift is the one judgement the report cannot make — it needs the suite to have
actually run (step 4 below).

## Two ways the report can mislead, and how to read it

- **`totals.total == 0` is not full coverage.** It means the declared model matched nothing
  in this scope — no minting document was found. Recent `quire` prints `no rows matched`
  rather than a percentage and fails `--strict` (quire-rs FR-050-AC-14); an older build
  printed `0/0 rows backed (100%)` and exited 0. Treat a zero denominator as **no data**,
  say so in `## Coverage`, and fall back (below). It is not a `PASS`.
- **An empty `unbacked_rows` proves nothing on its own.** It lists *reference rows* — cells
  that point at trace ids. A repo whose module declares no such references has an empty list
  regardless of how many tests are tagged. Read `groups` / `totals` alongside it.

## Fallback: a repo on an older module set

`quire coverage` exits non-zero with a distinct diagnostic when no active module declares a
`traceability:` model (FR-050-AC-9). That is a real repo state, not an error to swallow —
`spec-artifacts-process` only began declaring the model at the release carrying FR-004.

When the command refuses, or reports a zero denominator, fall back to the grep index below
and **say which path ran** in the SpecReview's `## Coverage` section:

```
Reconciliation: quire coverage (module spec-artifacts-process 0.11.0)
Reconciliation: grep fallback — no active module declares a traceability model
```

A finding derived from the fallback is weaker and should be read as such: grep matches a tag
wherever it sits in a file, including places the engine will not bind it. In quoin, ~15 tags
sat above a `describe(` block, which registers no symbol — greppable, and invisible to the
engine (agent-ix/quoin#61). **Never present a grep count as a coverage figure** without
naming it as a fallback.

### The fallback index

Grep the test tree across `.py` / `.ts` / `.rs` for every form:

| Form | Example |
| --- | --- |
| Docstring AC trace | `FR-007-AC-01` inside a test docstring `Description:` block |
| Docstring `Trace:` | `Trace: FR-001` |
| Module docstring | `"""Tests for FR-017: Context Management."""` |
| Test-case id comment / name | `# TC-041`, or a test whose name maps to `TC-041` |

Build `{tag → [test file :: test name]}`, then reconcile against the matrix by hand for the
same four finding kinds.

## Optionally run the suite

If the user wants execution evidence rather than static reconciliation, run `make test` and
note failures and skips against matrix rows. This is what turns marker drift from a guess
into a finding. Don't block on it if the environment can't run tests; say so.

## Rollup

`totals.backed` / `totals.total` feeds `## Coverage`, with the per-document breakdown from
`groups` where it helps. Report the numbers the tool produced — do not recompute them, and
do not quote a figure from an earlier run whose provenance you cannot state.

## Output of this step

Findings (unbacked rows, status lies, untracked tests, marker drift) with `Refs` from the
report, the backed/total count, and which reconciliation path ran.

## Notes

- A backed row proves *traceability*, not *correctness* — whether the test is a good test is
  Step 4 (underspecified/stub) and Step 5 (semantic). Keep this step about presence + trace.
- The command performs no network or service I/O and executes none of the code it reads
  (FR-050-CON-2, FR-051-CON-1), so it is safe to run in any repo.
