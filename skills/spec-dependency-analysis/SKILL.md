---
name: spec-dependency-analysis
description: Analyze dependencies and separate enablement from feature work.
---

# Dependency & Ordering Analysis

Identify prerequisite relationships between requirements so work can be sequenced. The deliverable is a directed acyclic graph (DAG) of FRs/StRs/NFRs in **logical order**, not a schedule.

## When to use

- Before `spec-to-plan` — `spec-to-plan` consumes the DAG.
- After all FRs have been written and reviewed.
- Whenever new FRs are added that may unblock or block existing work.

## Process

1. **Identify prerequisite edges.** For each FR/StR/NFR, list every other requirement that MUST be satisfied before this one can be implemented. Express each as `A → B` (A is prerequisite to B).

2. **Classify each requirement.**
   - **Enablement** — frameworks, abstractions, scaffolding, schema/migration, shared libs. Has no business-visible behavior on its own.
   - **Feature** — business or user-visible behavior.

   **Rule**: Enablement requirements MUST be tasked before any feature that depends on them.

3. **Verify acyclicity.** If any cycle is found, the spec is malformed — break the cycle by introducing a separation FR or merging the cyclic requirements.

4. **Produce the deliverable.** Use the template below.

## Deliverable template

Record output in `spec/analysis/dependency.md` (or as a section in `spec/spec.md` § 7 if your project keeps analysis inline).

```markdown
# Dependency Analysis

## Classification

| Requirement | Class | Rationale |
|-------------|-------|-----------|
| FR-001 | Enablement | Defines the auth middleware all feature FRs use |
| FR-002 | Enablement | Database schema for user records |
| FR-003 | Feature | Login flow — depends on FR-001, FR-002 |
| FR-004 | Feature | Password reset — depends on FR-001, FR-002 |

## Dependency Graph

​```mermaid
graph TD
  FR-001[FR-001: Auth middleware]
  FR-002[FR-002: User schema]
  FR-003[FR-003: Login flow]
  FR-004[FR-004: Password reset]
  FR-001 --> FR-003
  FR-002 --> FR-003
  FR-001 --> FR-004
  FR-002 --> FR-004
​```

## Topological Order (suggested implementation sequence)

1. FR-001, FR-002 (enablement, parallelizable)
2. FR-003, FR-004 (features, parallelizable after enablement)

## Cycles

None detected.
```

## Acceptance Criteria

- Every FR/StR/NFR appears in the classification table exactly once.
- Every edge in the graph corresponds to an explicit prerequisite — no "soft" dependencies.
- No cycles.
- Topological order respects: all enablement before any feature that depends on it.

## Output contract

This skill emits one **`SpecReview`** under `reviews/YY-MM-DD-<slug>.md` with
`analysis: dependency` in its frontmatter.

Stated here because nothing checked it. The module declares an `analysis`
vocabulary of eleven values; five of them named a skill that never said it
emitted under them, so a skill and its declared output were unlinked and drift
between the two was invisible. `tests/skill-contracts.test.ts` now asserts the
pairing in both directions.

The document must carry a `## Summary` and a `## Findings` table — the two
sections `SpecReview`'s `body_extraction` requires — and must pass
`quire validate`.
