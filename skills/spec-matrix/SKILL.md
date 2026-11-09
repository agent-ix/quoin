---
name: spec-matrix
description: Read the computed Test Matrix (`quire matrix` / `quoin matrix`) and close its gaps by adding criterion-id trace tags to the tests that assert each untagged criterion. Never hand-writes a matrix.
---

# Close Test Matrix Gaps

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

The Test Matrix is **computed**, never written. `quire matrix` derives it on every run from
the spec's acceptance criteria and the trace tags in the source tree; `quoin matrix` adds
whether a passing run backs each criterion. This skill reads that matrix and makes the one
change that moves it: a trace tag on a test.

Do not create or edit `spec/matrix.md`, `spec/tests.md` or `spec/test-cases/`, and do not
mint `TC-` ids. A test binds to a criterion by that criterion's own id.

## 1. Read the matrix

```bash
quire matrix --scope . --format json
```

`--scope` is the repository root: spec documents are read from `<scope>/spec` and trace tags
from the rest of the tree.

For run evidence as well, use `quoin matrix --repo . --json`. If `quoin matrix --repo . --json` fails, use
`quire matrix --scope . --format json` above and read the static status alone.

Each criterion carries one static status:

| Status | Meaning | Action |
| --- | --- | --- |
| `tagged` | At least one test that is not ignored carries the id. | None. |
| `untagged` | No test carries the id. | Find or write the test (step 2). |
| `tagged-by-ignored-test` | Every test carrying the id is ignored or skipped. | Un-ignore it, or write a test that runs. |
| `method-without-symbol` | The criterion's method is one the module declares mints no source symbol (`Inspection`, `Analysis`, `Manual`, `Eval`). | None — no tag is expected. |

## 2. Tag the test that asserts the criterion

For each `untagged` criterion, read its statement and find the test that actually asserts it.
Tag that test with the criterion id. The canonical markers are the ones the module
manifest declares:

```rust
use ix_trace_rs::trace; // the attribute is a macro; the crate must be a dev-dependency

#[trace("FR-012-AC-3")]
#[test]
fn rejects_an_unpinned_import() { … }
```

```python
@pytest.mark.trace("FR-012-AC-3")
def test_rejects_an_unpinned_import(): ...
```

- A `Trace: FR-012-AC-3` comment line (`/// Trace:` in Rust) no longer binds (quire engine
  0.55): ids bind only from the marker attribute, and `quire coverage` reports a comment id as
  `trace-id-outside-marker` naming the replacement form. A production item that implements a
  requirement carries `#[implements("FR-012")]`; a test, bench or fuzz target carries `#[trace(...)]`.
- Tag the **criterion** (`FR-012-AC-3`), not the requirement. For AC-to-TC-to-code backing
  (`quire coverage`) the test must also carry the TC id of the `spec/matrix.md` row that cites
  the criterion: `#[trace("FR-012-AC-3", "TC-345")]`. A criterion tag alone is reported
  `criterion-tagged-outside-row` ("expected a same-test tag for one of `TC-345`") and gives no
  backing.
- Several ids on one marker are separate quoted arguments: `#[trace("FR-012-AC-3", "FR-012-AC-4")]`.
- Only tag a test that would fail if the criterion were broken. A tag on a test that does not
  assert the criterion is coverage inflation, and `gap-analysis` reports it as such.
- If no test asserts the criterion, write one. If the criterion cannot fail, fix the
  criterion (`spec-criterion-strength-analysis`) rather than tagging something.

## 3. Re-run and confirm

```bash
quire matrix --scope . --format tsv
```

The criteria you tagged read `tagged`. A tag that names an id no criterion declares binds to
nothing; `quire coverage --scope . --json` lists it under `untracked_symbols`. Fix the id.

## Criteria worth tagging

A criterion is only mappable when it states one obligation. Before tagging, run the EARS
grammar check and fix `non-singular` or `unclassifiable` statements first — a test cannot
cleanly assert two `shall` clauses at once:

```bash
quire validate --scope . "spec/**/*.md" --summary
```
