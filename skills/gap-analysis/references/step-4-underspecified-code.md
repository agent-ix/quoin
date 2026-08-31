# Step 4: Underspecified Code (reverse gap)

**Goal**: Find the *reverse* gap — code and behavior that exist with **no owning
requirement**, plus stubs that masquerade as complete. Steps 2–3 verify spec→code/test
coverage; this step verifies code→spec, catching scope that was implemented but never
specified.

This reference is the authoritative, self-contained reverse-gap procedure. Do not
consult a private repository, user-home path, or separately installed skill for the
detection heuristics below.

## A. Untraced behavior (code with no requirement)

1. **Inventory the surface.** List the component's real behaviors — public
   functions/methods, CLI commands/flags, HTTP endpoints, events, config knobs.
2. **Trace each to a requirement.** A behavior is *traced* if it maps to an StR/US/FR/NFR
   (via the matrix `Traces To`, a requirement that describes it, or a Task `references`
   edge that owns it).
3. **Flag the untraced.** Behavior with no owning requirement → finding:
   - `high` if it is user-visible or security/data-affecting (an endpoint, a CLI command, a
     destructive operation) — unspecified surface area.
   - `medium` for internal-but-meaningful logic with no requirement.
   - `low` for trivial helpers/glue.
   `Refs` = the code location (`path::symbol`).

## B. Implied requirements from code patterns

Map suspicious patterns to the requirement that *should* exist (per the
implementation-gap-analysis table):

| Code pattern | Missing requirement type |
| --- | --- |
| Defensive code (try/catch, null guards) | Robustness (NFR) |
| Ambiguous identity/equality logic | Integrity (StR) |
| Side effects / IO in core logic | Determinism (StR) |
| Recursion / unbounded structures | Topology / limits (NFR) |
| Unmapped constraints | Traceability (FR/StR/NFR) |

Each unstated-but-implemented constraint → `medium` finding.

## C. Stubs masquerading as complete

Scan **source** (not just tests) for hollow implementations the plan/matrix may report as
done:

| Stub | Detect | Severity |
| --- | --- | --- |
| Tiny file (≤5 lines, excl. `__init__`) | line count | high |
| `pass` / placeholder return (`{}`,`[]`,`None`,`"not implemented"`) body | grep/AST | high |
| Protocol/ABC-only with no concrete impl | structure | high |
| Re-export-only module | structure | medium (may be intentional) |
| Trivially-covered stub (≤5 lines @ 100% cov) | coverage + size | medium |

A stub behind a `done` task or a ✅ matrix row is a `high` finding (false completion).

Inspect test files as well as source. A passing test can still be a stub:

| Test stub | Detect | Severity |
| --- | --- | --- |
| Empty body or unconditional skip | `pass`, empty callback, or skip without a tracked reason | high |
| No behavioral assertion | no assertion, rejection, snapshot, or observable-effect check | high |
| Weak-only assertion | only existence/type checks for behavior with a stronger contract | high |
| Mock-everything | all meaningful collaborators replaced so production behavior is never exercised | high |
| Import-only smoke test | verifies only that a module imports | medium |

Do not flag an abstract declaration, generated file, intentional compatibility re-export,
or a test whose assertion is expressed through the repository's harness merely because it
matches a textual pattern. Confirm the behavior is hollow before recording a finding.

## D. Coverage inflation

Treat coverage as evidence only after checking what the covered lines do:

- A source file with five or fewer substantive lines and 100% coverage can still be a
  trivially covered stub.
- A test that mocks a source module which is itself hollow is circular evidence, not
  implementation coverage.
- A threshold dominated by imports, declarations, or re-exports does not establish that
  the promised behavior exists.

Record these as `high` when they support a `done` task or ✅ matrix claim, otherwise
`medium`. Cite both the hollow source and the test or coverage artifact.

## Output of this step

Findings for untraced behavior, implied-but-missing requirements, source/test stubs, and
coverage inflation, each with `Refs` = concrete code/test location. Include counts of
inventoried behaviors, untraced behaviors, source stubs, and test stubs in `## Coverage`.

## Notes

- This is the bridge to better specs: untraced behavior usually means a requirement should
  be authored (`/specify`) — but gap-analysis only **reports** it; it does not author specs.
