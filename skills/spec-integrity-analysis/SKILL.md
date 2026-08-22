---
name: spec-integrity-analysis
description: Perform quality gate analysis on ISO-style specifications to ensure completeness, consistency, and atomicity.
---

# Specification Integrity Analysis

This skill performs a quality gate analysis on specifications (User Stories, FR/SR, StR, NFR) before task generation.

## Process

1.  **Completeness Analysis**:
    -   Verify every User Story (US) maps to >= 1 Functional/System Requirement (FR/SR).
    -   Verify every FR/SR maps to >= 1 Stakeholder Requirement (StR).
    -   Verify every FR/SR maps to >= 1 verification method.
    -   Verify every NFR is explicitly scoped and referenced by affected FR/SR.
    -   **Deliverable**: Traceability matrix (US -> FR/SR -> StR -> Verification).

2.  **Consistency & Conflict Analysis**:
    -   Identify contradictory constraints (e.g., latency vs security).
    -   Identify duplicated requirements with different wording.
    -   Identify hidden assumptions embedded in prose.
    -   **Rule**: No requirement may have more than one valid interpretation.

    ### Hidden Assumption Probes

    For every FR, explicitly check for these patterns:

    | Pattern | Probe Question |
    |---------|---------------|
    | FR delegates to an external CLI (git, gh, cookiecutter, etc.) | Is there an NFR declaring the tool's minimum version, detection method, and user-facing error when absent? |
    | FR performs a lookup over multiple sources (registries, plugins, taps) | Is there an explicit tie-breaking policy when two sources return the same key? (first-wins, last-wins, error, merge — must be a stated decision) |
    | FR calls an external API that returns paginated results | Does an NFR state whether full enumeration is required? If a cap is acceptable, is it documented as a known limitation? |
    | FR calls an external API in a loop or concurrently | Does an NFR declare max concurrency, rate limit awareness, retry policy, and that errors must surface (not be silently swallowed)? |
    | FR involves a scaffolding or generation command | Are both an interactive mode (human at terminal) and a scripted/CI mode explicitly specified with their flags? |
    | FR calls an authenticated external API | Does an NFR declare the required auth scopes and mandate a clear user-facing error when scope is insufficient (not silent empty results)? |
    | FR depends on a package or service not yet implemented | Is the interim fallback and eventual resolution chain documented? Is the hardcoded default traceable to a known stub? |

3.  **Atomicity & Testability Analysis**:
    -   Ensure each FR/SR/StR defines a *single* behavior/obligation.
    -   Ensure behavior is externally observable.
    -   Ensure behavior is verifiable by test, inspection, analysis, or demonstration.
    -   **Action**: Flag non-testable requirements for rewriting or splitting.

## Failure Domain Check
Before finalizing, verify against `spec-failure-domain-analysis`:
- Extension failures, Identity keys, Evaluation purity, Topological robustness.

## Output contract

This skill emits one **`SpecReview`** under `reviews/YY-MM-DD-<slug>.md` with
`analysis: integrity` in its frontmatter.

Stated here because nothing checked it. The module declares an `analysis`
vocabulary of eleven values; five of them named a skill that never said it
emitted under them, so a skill and its declared output were unlinked and drift
between the two was invisible. `tests/skill-contracts.test.ts` now asserts the
pairing in both directions.

The document must carry a `## Summary` and a `## Findings` table — the two
sections `SpecReview`'s `body_extraction` requires — and must pass
`quire validate`.
