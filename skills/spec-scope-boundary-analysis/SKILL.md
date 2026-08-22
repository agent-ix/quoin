---
name: spec-scope-boundary-analysis
description: Define system boundaries and allocate responsibilities for ISO-style specifications.
---

# Scope & Boundary Analysis

Define what the system owns, what it consumes from outside, and which component owns each requirement.

## When to use

- Once the StR/FR set is stable but before tasking.
- When a new external dependency or component is introduced.
- During `spec-app-review` for application-type specs that span multiple components.

## Process

1. **System Boundary Definition.**
   - Enumerate in-scope responsibilities (what this system guarantees).
   - Enumerate external dependencies (what it consumes and trusts).
   - For each dependency, mark whether its behavior is **assumed** (we don't verify it) or **guaranteed** (we verify via contract test).

2. **Responsibility Allocation.** For each FR/StR/NFR, assign:
   - An **owning component or subsystem**.
   - A **responsibility class**:
     - `core` — domain logic specific to this system.
     - `infrastructure` — persistence, transport, scheduling, storage.
     - `cross-cutting` — auth, logging, metrics, error handling, audit.

   This step defines *where* work belongs, not *how* it is done.

3. **Produce the deliverable** using the template below.

## Deliverable template

Record output in `spec/analysis/scope-boundary.md`.

```markdown
# Scope & Boundary Analysis

## System Context

​```mermaid
flowchart LR
  user([User])
  ext1[(External: Identity Provider)]
  ext2[(External: Audit Log Sink)]
  subgraph SUT [System Under Spec]
    api[API Gateway]
    core[Core Service]
    db[(Postgres)]
  end
  user --> api --> core --> db
  core -->|assumed| ext1
  core -->|guaranteed via contract test| ext2
​```

## In-Scope Responsibilities

- Authenticate and authorize all inbound API requests.
- Persist domain entities X, Y, Z with the schema defined in FR-010..FR-012.
- Emit audit events on every state transition.

## External Dependencies

| Dependency | Type | Assumed or Guaranteed | Contract |
|------------|------|------------------------|----------|
| Identity Provider | OIDC | Assumed | Provider's published spec |
| Audit Log Sink | HTTP | Guaranteed | IT-005 contract test |

## Responsibility Allocation

| Requirement | Owning Component | Class |
|-------------|------------------|-------|
| FR-001 (auth middleware) | API Gateway | cross-cutting |
| FR-010 (user entity) | Core Service | core |
| FR-020 (postgres migration) | Core Service | infrastructure |
| NFR-003 (request log) | API Gateway | cross-cutting |
```

## Acceptance Criteria

- Every FR/StR/NFR is assigned exactly one owning component and one responsibility class.
- Every external dependency is marked `assumed` or `guaranteed` with a named contract.
- The system context diagram shows every external actor named in the spec.
- No "TBD" entries in the allocation table.

## Output contract

This skill emits one **`SpecReview`** under `reviews/YY-MM-DD-<slug>.md` with
`analysis: scope-boundary` in its frontmatter.

Stated here because nothing checked it. The module declares an `analysis`
vocabulary of eleven values; five of them named a skill that never said it
emitted under them, so a skill and its declared output were unlinked and drift
between the two was invisible. `tests/skill-contracts.test.ts` now asserts the
pairing in both directions.

The document must carry a `## Summary` and a `## Findings` table — the two
sections `SpecReview`'s `body_extraction` requires — and must pass
`quire validate`.
