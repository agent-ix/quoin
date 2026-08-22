---
name: spec-failure-domain-analysis
description: Identify unstated failure modes, identity confusion, purity gaps, and topological edge cases in ISO-style specifications.
---

# Spec Failure Domain Analysis

Run this skill during spec review to catch common omissions before implementation.

## When to Use
- After drafting FR/NFR/StR requirements
- Before `spec-to-plan` (part of its readiness gate)
- During peer review of specifications

## Checklist

### 1. Extension Points (Trust Boundaries)
For each callback, hook, observer, or plugin interface:
- [ ] Is failure behavior defined? Options:
  - **Strict**: Failure aborts operation (data integrity)
  - **Resilient**: Failure is logged/suppressed (availability)
  - **Configurable**: User chooses policy

### 2. Entity Identity
For each core domain entity (State, Event, User, etc.):
- [ ] Is the uniqueness key explicit?
  - Bad: "A set of states"
  - Good: "States unique by `name` property"

### 3. Evaluation Purity
For each user-supplied logic (guards, filters, validators):
- [ ] Are side-effects constrained?
  - Document if read-only is required for simulation/reasoning

### 4. Topological Robustness
For each graph/tree algorithm (search, path-finding):
- [ ] Is termination guaranteed on cycles?
- [ ] Are worst-case structures considered? (deep nesting, disconnected nodes)

## Output
Document any missing constraints as proposed additions:
- **NFR**: Non-functional (resilience, performance)
- **StR**: Structural (integrity, purity)
- **FR**: Functional (if behavior is undefined)

## Output contract

This skill emits one **`SpecReview`** under `reviews/YY-MM-DD-<slug>.md` with
`analysis: failure-domain` in its frontmatter.

Stated here because nothing checked it. The module declares an `analysis`
vocabulary of eleven values; five of them named a skill that never said it
emitted under them, so a skill and its declared output were unlinked and drift
between the two was invisible. `tests/skill-contracts.test.ts` now asserts the
pairing in both directions.

The document must carry a `## Summary` and a `## Findings` table — the two
sections `SpecReview`'s `body_extraction` requires — and must pass
`quire validate`.
