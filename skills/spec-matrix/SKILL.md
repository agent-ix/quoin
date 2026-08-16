---
name: spec-matrix
description: Build and maintain the requirements Test Matrix ensuring 100% coverage.
---

# Build Test Matrix

Use this skill to create the requirements Test Matrix and ensure comprehensive coverage.

## Rules (The Six Requirements)

1.  Coverage: Every Acceptance Criterion (AC) must have ≥1 Test Case (TC).
2.  Option Permutation: Test all valid option combinations.
3.  Constraint Boundary: Test min, max, below-min (fail), above-max (fail).
4.  Error Path: Test all documented error conditions.
5.  State Transition: Test all valid state transitions (if applicable).
6.  Edge Case: Identify and test extreme/unusual scenarios.

## EARS precondition (atomic, mappable requirements)

Coverage (Rule 1) is only meaningful when each requirement states **one**
obligation — a statement packing two `shall` clauses cannot map cleanly to one
Acceptance Criterion, so its matrix row is ambiguous. Before building the
matrix, run the EARS requirement-grammar check:

```bash
quire validate --scope . "spec/**/*.md" --summary
```

> **`--scope` is the repository root, and must be passed explicitly.** Since quire-cli
> v0.16.0 (quire-rs CR-045) the command derives **two roots** from it and never
> interchanges them: spec documents are read from `<repo>/spec` only, trace tags from
> the source tree at `<repo>` excluding `spec/`. A repo with no `spec/` exits with a
> diagnostic naming the missing document root rather than scanning the whole tree, and a
> matrix outside `spec/` (a fixture, a `plan/` copy) mints nothing. A relative glob
> resolves under `--scope` only in scoped mode (no `--module`); with `--module` it
> resolves against the process working directory, and an omitted `--scope` defaults to
> `.` — so a run launched from a parent directory validates the **wrong tree** and exits
> 0 for whatever it matched. Check `quire --version` ≥ 0.16.0 before relying on any of
> this; ≤ 0.15.0 has the pre-split traversal semantics.


Treat these `[ears:…]` findings as **un-mappable requirements to fix first**,
not coverage you can claim:

-   `non-singular` — split the statement into one requirement per `shall`, then
    map each to its own AC/TC.
-   `unclassifiable` / `missing-subject` — the obligation is unclear; clarify the
    requirement before authoring a TC against it.

Do not mark the matrix ✅ Complete while non-singular or unclassifiable
requirements remain — the coverage they appear to have is illusory. (The check
is advisory; this skill treats it as a quality gate for the matrix.)

## Process

1.  Initialize:
    -   Use template `assets/test-matrix-template.md`.
    -   Target: Correct location.
        -   Single Repo: `spec/tests.md`
        -   Multi-Repo: `specs/<category>/<component>/spec/tests.md`
2.  Traceability:
    -   Map `StR` -> `TC`.
    -   Map `US` -> `TC`.
    -   Map `FR` -> `TC`.
    -   Map `NFR` -> `TC`.
    -   Map `C` (Constraints) -> `TC`.
3.  Enumerate: List all Test Cases with ID, Title, Type, Priority, Status.
    See `Test Case Summary` below for the `Type` and `Priority` vocabularies —
    both are validated, so a value outside the set fails `quire validate`.
4.  Define Detailed TCs (Optional):
    -   For complex tests, create at:
        -   Single Repo: `spec/test-cases/TC-XXX.md`
        -   Multi-Repo: `specs/<category>/<component>/spec/test-cases/TC-XXX.md`
5.  Verify: Ensure all 6 rules are satisfied.
6.  Status: Mark as ✅ Complete only when all rules are met.

## Test Case Summary

The `## Test Case Summary` table is structurally validated by `quire validate`
against the `TestMatrix` archetype (`spec-artifacts-process` FR-003). Columns are
exactly `Test ID | Title | Type | Priority | Traces To | Status`.

The vocabularies below are **not owned by this skill**. The single source is
`spec_artifacts_process/manifest.yaml` (`traceability.vocabularies.test_type`,
mirrored into `column_choices`); if this list and the manifest ever disagree, the
manifest wins and this section is the bug.

### `Type` — exactly one value per row

Pick by *what makes the test fail*, not by which framework runs it.

| Value | Use when |
|---|---|
| `Unit` | One function/module in isolation, example-based. The default. |
| `Integration` | Two or more components, or a real dependency (DB, service, browser). |
| `E2E` | The whole system through its outermost interface, as a user drives it. |
| `Property` | The criterion holds **over a domain of inputs**, and the test generates them. See below. |
| `Fuzz` | Untargeted/mutational input generation looking for crashes, not a stated property. |
| `Benchmark` | The criterion is a latency/throughput number; the test measures it. |
| `Static` | Proved by analysis over the source — a lint, an audit script, an architecture check. No runtime. |
| `Compile` | The failure mode *is* a compile error (type-level guarantee, `forbid(unsafe_code)`). |
| `Snapshot` | Byte/DOM-identity against a stored artifact. |
| `Manual` | Verified by a human procedure; no automated gate. |

**When `Property` rather than `Unit`.** Read the acceptance criterion and ask
whether it quantifies. A criterion that names one input and one expected output
is a `Unit` example. A criterion that asserts something for *every* input, or
relates two runs to each other, is property-shaped:

- **Invariant** — "output is always sorted", "the count never exceeds `max_size`"
- **Round-trip** — "parse then render returns the original bytes"
- **Idempotence** — "applying it twice equals applying it once"
- **Metamorphic** — "reordering the inputs does not change the result"
- **Oracle** — "the fast path agrees with the reference implementation"

Words like *always*, *never*, *for any*, *regardless of*, *preserves*,
*round-trips*, *deterministic*, *order-independent* are the signal. Where a
criterion is property-shaped, prefer `Property` — a generated test discriminates
the claim, a single example only witnesses it.

`Property` is a first-class value: do not downgrade a property-shaped criterion
to `Unit` because the repo has no generator yet. Author the row, and let the
missing generator show up as a gap.

### `Priority`

`P0` | `P1` | `P2` | `P3` | `P4`.

### `Status`

`✅` complete · `⚠️` partial · `❌` failed · `🚧` in progress · `⛔` retired.
A marker may be followed by text (`✅ Complete`); the marker must come first.

### `Traces To`

Comma-separated requirement ids (`FR-012-AC-3`, `US-004-AC-1`), ranges allowed
(`FR-012-AC-1..3`), optional trailing ` (note)`. Trace to the **criterion**, not
the requirement, and never to another `TC`.

## React Components (Conditional)

For React projects, add a Storybook Test Matrix section.

See `skills/review-react/SKILL.md` for:
- Story file requirements per component
- AC trace code format in story JSDoc
- Coverage matrix template (FR → AC → Story)

## Integration Test Matrix (Conditional)

If ANY requirement specifies integration testing, include this section.

### When Required

- FR/NFR annotation: `integration_test: true` or mentions integration verification
- Cross-project interactions (other services in your org)
- Interactions with local-provided services (Redis, Postgres, message bus)
- UI component browser testing (React → Storybook)

### Section Contents

Use template section `## Integration Test Matrix` with:
1. **Purpose column**: What integration is being tested
2. **Target column**: External system/service/project
3. **Type column**: `service` | `browser` | `event` | `database`
4. **Test Cases**: Extensive functional coverage, not just smoke tests

### Coverage Expectations

- Extensively test functionality (multiple scenarios per integration)
- Include happy path + error conditions + edge cases
- For browser tests: visual verification + interaction flows

## Markers

-   ✅ Complete
-   ⚠️ Partial
-   ❌ Missing
-   🚧 In Progress
-   ⛔ Retired
