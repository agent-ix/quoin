# Step 3: Test-Matrix Verification

**Goal**: Prove the Test Matrix is *real* — every Test Case it claims is backed by an actual
test in the suite, identified by a matching **tracking tag** in the test code. This is the
heart of gap-analysis: a matrix row marked ✅ means nothing unless a tagged test exists.

## The matrix shape

Built by `spec-matrix` (`quoin/skills/spec-matrix/SKILL.md`). Key tables:

- **Test Case Summary** — `Test ID | Title | Type | Priority | Traces To | Status`, where
  `Test ID` = `TC-xxx`, `Traces To` = `FR-XXX-AC-X` etc., `Status` ∈ `✅ ⚠️ ❌ 🚧 ⛔`.
- **Per-requirement coverage** — StR/US/FR/NFR → AC → TC → Status tables.

## Tracking-tag patterns in test code

Tests declare which requirement/test-case they cover via tags in the code. The ecosystem
uses these forms (grep for all of them, across `.py`/`.ts`/`.rs`):

| Form | Example |
| --- | --- |
| Docstring AC trace | `FR-007-AC-01` inside a test docstring `Description:` block |
| Docstring `Trace:` | `Trace: FR-001` |
| Module docstring | `"""Tests for FR-017: Context Management."""` |
| Test-case id comment / name | `# TC-041`, or a test whose name maps to `TC-041` |

## Process

1. **Parse the matrix.** Extract the set of declared Test Cases `{TC-xxx → (Traces To, Status)}`.
2. **Index the suite.** Grep the test tree for every tracking tag form above; build
   `{tag → [test file :: test name]}` for `TC-xxx`, `FR-xxx`, `FR-xxx-AC-x`.
3. **Reconcile, producing findings:**
   - **Unbacked matrix row** — a `TC-xxx` (or its `Traces To` requirement) with **no**
     matching tag in any test → `high` finding (the matrix overclaims coverage).
   - **Status lie** — a row marked `✅ Complete` but its test is missing, skipped
     (`pytest.skip`), or its requirement has no tagged test → `high`.
   - **Untracked test** — a real test that exercises behavior but carries **no** tracking
     tag and is **absent** from the matrix → `medium` (coverage exists but isn't traced).
   - **Marker drift** — matrix `Status` inconsistent with reality (e.g. `🚧` for a passing
     tagged test, or `✅` for a `⚠️`-quality test) → `low`.
4. **Optionally run the suite.** If the user wants execution evidence (not just static
   tag-matching), run `make test` and note failures/skips against matrix rows. Don't block
   on this if the environment can't run tests; say so.
5. **Rollup.** Count matrix Test Cases backed-by-tagged-test / total — feeds `## Coverage`.

## Output of this step

Findings (unbacked rows, status lies, untracked tests, marker drift) with `Refs` =
`TC-xxx` / `FR-xxx` / test path, plus the backed/total count.

## Notes

- A tag match proves *traceability*, not *correctness* — whether the test is a good test is
  Step 4 (underspecified/stub) and Step 5 (semantic). Keep this step about presence + trace.
