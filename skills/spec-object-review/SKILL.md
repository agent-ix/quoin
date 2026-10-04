---
name: spec-object-review
description: Audit spec domain objects, cross-references, and entity completeness across single or multi-repo ecosystems.
---

# Spec Object Review

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

Audit a codebase's spec domain objects for completeness, cross-reference integrity, and alignment with the object-type-guide format.

## When to Use

- After initial spec creation to verify domain coverage.
- When integrating a new service into an existing ecosystem.
- When fixing spec-code misalignments or tracing integration bugs.
- As a periodic health check on spec quality.

## Process

### 1. Inventory

Scan all `spec/` directories in scope (single repo or ecosystem-wide):

```bash
find spec/ -name "*.md" -path "*/functional/*"
```

For each FR, extract frontmatter fields: `id`, `title`, `type`, `object`.

### 2. Cross-Reference Audit

For every FR with a Traceability / Dependencies section:

- **Verify upstream refs exist**: If FR-A says it depends on `repo-B FR-003`, confirm `repo-B/spec/functional/FR-003-*.md` exists.
- **Verify bidirectional tracing**: If FR-A references FR-B as upstream, FR-B should reference FR-A as downstream (or vice versa).
- **Check endpoint paths match code**: For any API endpoint documented in a spec, grep the source code to confirm the path is correct. Pay attention to router prefixes that may not appear in the FR but are applied at app mount time.

**Common Pitfalls:**
- Router prefixes (e.g., `/auth` prefix) silently changing the actual path vs what the FR documents.
- References using vague service names ("identity") instead of specific FR IDs ("identity FR-006").
- `depends_on` in master spec frontmatter being empty when runtime dependencies exist.

### 3. Domain Object Completeness

Check whether domain objects defined inline in specs have proper dedicated FRs with `object:` frontmatter.

**Scan for inline objects:**

| Pattern in spec | Missing Domain FR Type |
|----------------|----------------------|
| `### Data Models` table with field/type columns | `entity` or `value_object` |
| API Request/Response JSON bodies in `## API Contract` | `value_object` |
| `class X(SQLModel)` or Pydantic model in code without FR | `entity` |
| `class X(str, Enum)` or status fields | `value_object` or `state_machine` |
| Multi-step behavior spanning services | `process` |
| Env var / configuration table in spec.md | `configuration` |
| `stateDiagram-v2` in spec.md | `state_machine` |

**The Semantic Payload Trap:** Inline JSON bodies inside of an endpoint's API Contract section are the #1 cause of cross-repo semantic drift. Even if the syntax matches (e.g. `{ "username": "string" }`), the *semantics* drift (`email` vs `account_name`). Any structured payload passed across a bounded context MUST have a dedicated `value_object` FR to anchor its field semantics.

**Reference**: See [object-type-guide.md](references/object-type-guide.md) for the full type taxonomy.

### 4. Frontmatter Validation

All FRs must use standard frontmatter:

```yaml
---
id: FR-XXX
title: "Descriptive Title"
type: FR
object: entity          # Only if FR defines a domain object
relationships:          # Structured array of cross-references
  - target: "ix://org/repo/TargetFR"
    type: "depends_on"
    cardinality: "1:1"
---
```

**Check for:**
- `type: functional` instead of `type: FR` (non-standard value).
- Missing `object:` on FRs that clearly define domain objects.
- `domain:` or `status:` fields that aren't part of the standard schema.
- Missing `relationships:` array in the frontmatter when the body references upstream/downstream dependencies (such as an API, an event, a data entity, or a required permission).
- Dependencies defined as generic Markdown text lists or raw Mermaid diagrams instead of structured YAML relational objects.
- Relationships using generic `type: "depends_on"` when a more specific semantic linkage applies (e.g. `calls`, `publishes`, `consumes`, `triggers`, `reads`, `writes`, `requires`).

### 5. Rebuild Test

Ask yourself these questions. If the answer is "no" to any, there's a gap:

1. **Can I rebuild this system from spec alone?** Every entity, its fields, invariants, and relationships should be in a dedicated FR — not just inline markdown tables.
2. **Can I find how to perform operation X?** End-to-end flows (login, deploy, sync) should have `object: process` FRs that show the full sequence across services.
3. **Can I find how to integrate?** Each service should document what it consumes and what it exposes, with FR-level traceability.
4. **Can I troubleshoot?** Error handling sections should exist in every API endpoint and process FR.

### 6. Application-Level Check

For multi-service domains (e.g., Auth = auth-service + identity + auth-py + auth-fastapi + ts-auth-sdk):

- Is there a catalog fixture defining the Application entity?
- Does an application-level repo exist to own:
  - The bounded context / domain FR.
  - Cross-service process FRs.
  - The catalog fixture itself.
- Do individual service repos avoid duplicating domain-level concerns?

## Output

Produce a review report with:

| Priority | Finding | Repo | Impact |
|----------|---------|------|--------|
| P0 | Spec-code mismatch | repo | Incorrect documentation |
| P1 | Missing cross-reference | repo | Traceability gap |
| P2 | Missing `object:` frontmatter | repo | Parser incompatibility |
| P3 | Missing domain-typed FR | repo | Incomplete domain model |

## Tips from Real Reviews

- **Router prefix trap**: FastAPI router prefixes (e.g., `router = APIRouter(prefix="/auth")`) silently change paths. Always verify spec paths against the *mounted* path, not just the route decorator.
- **Vague traceability**: `| identity | Fetches user |` is weaker than `| identity FR-006 | Fetches user via internal lookup |`. Always use FR-level IDs.
- **Empty `depends_on`**: Client libraries almost always depend on the service they consume. An empty `depends_on: []` in a client SDK spec is a red flag.
- **Section number gaps**: Master specs jumping from §8 to §19 suggest template sections were deleted. Check if §8 (Domain Model) and §9 (Events and Signals) should be populated.
- **Shared entities**: If two FRs reference the same model (e.g., `AuthToken` used by both API Keys FR-004 and OAuth Clients FR-008), the model needs its own entity FR to avoid ambiguity.
- **Semantic mismatch trap**: Even when endpoint signatures align, field semantics can drift across bounded contexts (e.g., UI sends `email` mapped to SDK's `username` field, but Backend strictly expects an account name). Always cross-check the *meaning* and *format* of data passed across boundaries, especially in multi-repo systems like Auth.
