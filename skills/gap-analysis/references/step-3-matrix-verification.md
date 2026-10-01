# Test-Matrix Verification

**Goal**: Prove every acceptance criterion is backed by an actual test in the suite,
identified by a **trace tag** carrying the criterion's id. This is the heart of
gap-analysis: a criterion is covered when a test says so in code, not when a document does.

**The Test Matrix is computed, not read.** `quire matrix` derives it on every run from the
spec's criteria and the trace tags in the source tree; `quoin matrix` adds the run evidence
on top. Neither writes anything. A hand-written `spec/matrix.md` or `spec/tests.md`, where a
repository still has one, is not an input to this step: do not reconcile its rows or its
`Status` cells, and do not raise a finding because a repository has none.

**Scope: the whole repository, always.** This step runs identically in planless and
plan-assisted mode. `--scope` is the repository root and the criterion set is every
criterion the spec declares — never the subset a plan's tasks happen to touch. A supplied
plan neither selects criteria nor excuses an untagged one.

**This step does not grep.** The engine computes the matrix and this skill interprets it.
Severity and verdict stay here — the commands report and do not judge (quire-rs
FR-050-CON-1).

## Run the matrix

The evidence-backed matrix is the preferred input, because it carries the static axis
verbatim and adds whether a passing run backs each criterion. It needs a quoin
with the `--repo` flag:

```bash
quoin matrix --repo <project_root> --json
```

When `quoin matrix` is unavailable or refuses (for example, the repository's HEAD cannot be
resolved), read the static axis alone and say so in `## Coverage`:

```bash
quire matrix --scope <project_root> --format json
```

`--scope` is the repository root. The command derives **two roots** from it and never
interchanges them: spec documents are read from `<project_root>/spec` only, and trace tags
from the source tree at `<project_root>` excluding `spec/`.

Do **not** pass `--strict`. Whether a gap blocks is this skill's verdict rule (see the
[SpecReview artifact](step-6-specreview-artifact.md) step), not the command's exit code.

### What each criterion carries

| Field | Source | Values |
| --- | --- | --- |
| `static_status` (`status` under `quire matrix`) | trace tags | `tagged` · `untagged` · `tagged-by-ignored-test` · `method-without-symbol` |
| `binders` | trace tags | one `{path, line, column, qualified_name, kind}` per test bound to the criterion |
| `method` | the criterion's Verification cell | omitted when the criterion names none |
| `evidence_status` (`quoin matrix` only) | evidence store + auditor | `bound` · `stale` · `suspect` · `undischarged` · `no run evidence` |
| `evidence_detail` (`quoin matrix` only) | auditor | `findings`, `bindings`, `unevaluated` for that criterion |

`method-without-symbol` means the criterion's declared method is one the active module
declares mints no source symbol (`Inspection`, `Analysis`, `Manual`, `Eval` under
`spec-artifacts-process`), so no test tag is expected. It is neither a
gap nor coverage — count it separately.

## Stale tags

A test tagged with an id no criterion declares binds to nothing, and the matrix cannot show
it. Read those from the coverage report:

```bash
quire coverage --scope <project_root> --json
```

Use only `untracked_symbols` (`path`, `symbol`, `trace_id`) and `diagnostics` from it. Its
reference-row fields (`unbacked_rows`, `status_lies`) describe hand-written matrix rows, which
this step does not audit.

## Reconcile, producing findings

| Signal | Finding | Severity |
| --- | --- | --- |
| `untagged` | The criterion has no test carrying its id | `high` |
| `tagged-by-ignored-test` | Every test carrying its id is ignored or skipped, so nothing runs for it | `high` |
| `evidence_status: suspect` | The auditor distrusts the evidence (a changed statement, a mocked confirmation, vacuous or non-independent evidence) | `high` |
| `evidence_status: stale` | The bound run is behind HEAD, or its latest run failed | `medium` |
| `untracked_symbols` | A test tag naming an id no criterion declares — a typo, or a criterion that was renumbered | `medium` |

`Refs` for each finding is the criterion id, plus `path::qualified_name` from `binders` where
there is one, or `path::symbol` for an untracked symbol. Do not re-derive them.

`no run evidence` on every criterion means the evidence store has never been fed. Report it
once in `## Coverage`; it is not a per-criterion finding. `undischarged` on a `tagged`
criterion in a populated store means the tag exists and no passing run is recorded for it;
report it in `## Coverage` with its count, and raise it as a finding only when the user asked
for execution evidence.

## Two ways the matrix can mislead

- **Zero criteria is not full coverage.** Under `--format json`, `quire matrix` omits the
  `coverage_matrix` key entirely (the `No obligations matched this scope.` line appears only
  in markdown output), and `quoin matrix --json` returns a `reason` with no requirements,
  when the declared model matched nothing. Treat that as **no data**, say so in `## Coverage`, and fall back (below).
  It is not a `PASS`.
- **A non-empty `diagnostics` list** from `quire coverage` means a declaration selected
  nothing — an unreadable document, or a model with no trace targets. The matrix is then
  measuring less than it appears to.

## Fallback: a repo on an older module set

`quire matrix` and `quire coverage` exit non-zero with a distinct diagnostic when no active
module declares a `traceability:` model (FR-050-AC-9). That is a real repo state, not an
error to swallow.

When the commands refuse, or report zero criteria, fall back to a grep index of trace tags
across the test tree (`.py` / `.ts` / `.rs`: `Trace: FR-001-AC-2` doc lines,
`@pytest.mark.trace("FR-001-AC-2")`, criterion ids in test docstrings), reconcile it against
the criteria ids in `spec/` by hand, and **say which path ran** in `## Coverage`:

```
Reconciliation: quoin matrix (quoin <version>, quire <version>)
Reconciliation: quire matrix (quire <version>) — no run evidence read
Reconciliation: grep fallback — no active module declares a traceability model
```

A grep finding is weaker: grep matches a tag wherever it sits in a file, including places
the engine will not bind it (for example above a `describe(` block, which registers no
symbol). **Never present a grep count as a coverage figure** without naming it as a
fallback.

## Rollup

For `## Coverage`, count criteria by static status (`tagged` / `untagged` /
`tagged-by-ignored-test` / `method-without-symbol`) and, when `quoin matrix` ran, by evidence
status. Report the numbers the tool produced — do not recompute them, and do not quote a
figure from an earlier run whose provenance you cannot state.

## Output of this step

Findings (untagged criteria, ignored-only criteria, suspect or stale evidence, stale tags)
with `Refs` from the matrix, the per-status counts, and which reconciliation path ran.

## Notes

- A tagged criterion proves *traceability*, not *correctness* — whether the test is a good
  test is the [reverse-gap step](step-4-underspecified-code.md) and the
  [semantic review](step-5-semantic-review.md). Keep this step about presence and trace.
- `quire matrix` and `quire coverage` perform no network or service I/O and execute none of
  the code they read (FR-050-CON-2, FR-051-CON-1). `quoin matrix` reads the evidence store
  and writes nothing. All three are safe to run in any repo.
