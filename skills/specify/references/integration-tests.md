# Authoring Integration Tests (IT)

Load this when a request involves integration tests — i.e. the system talks to
external services (HTTP APIs, event buses, databases, third-party/cloud APIs).
The per-section shape comes from the `IT` template (`quoin write . --types IT`);
this file carries the *reasoning and reality rules* that the template can't
enforce.

## Scope

HTTP client integrations, event bus connections (Kafka, RabbitMQ), database
connections (Postgres, Redis), and external APIs. One test case per file,
sequentially numbered (`IT-XXX`), saved to `spec/integration/IT-XXX-*.md`.

## Reality rules — REAL by default

> Integration tests are **ALWAYS REAL** by default. Nothing is mocked unless the
> spec explicitly says so. An IT that only checks imports or constructs models
> without exercising real behavior is a **GAP**.

Every IT MUST include acceptance criteria from **both** categories:

- **Real Setup ACs** — the non-mocked environment is established: service running
  and healthy (real health endpoint), real dependencies available (DB, bus,
  external service), fixtures loaded into real stores, config applied through real
  loading paths.
- **Real Output ACs** — real output from real services: HTTP responses
  (status/headers/body), DB state changes (rows inserted/updated/deleted), events
  published to and consumed from a real bus, file-system state, SSE/WebSocket
  streams, LLM responses through real provider chains.

| Real (required)                                   | Not real (GAP)                 |
| ------------------------------------------------- | ------------------------------ |
| POST to endpoint, assert response body            | `assert MyModel is not None`   |
| Query DB after a write, verify rows changed       | `MyModel(field="value")`       |
| Publish event, consume from real bus, verify load | `assert EventSchema is not None` |
| GET /health, verify HTTP 200 + status body        | `assert app is not None`       |

### Mocking policy

- **Default: NO MOCKS.** ITs exercise real service boundaries.
- **Approved mock targets** only when the spec explicitly requires it: external
  third-party APIs (payment, auth, cloud) via test doubles over real HTTP;
  external services outside the test boundary via contract-faithful stubs.
- **Never mock**: HTTP transport, config loading, DB connections, file I/O,
  session storage, event publishing, health endpoints, WebSocket/SSE connections.
- When a mock IS used, the IT must state WHY the real service can't be used, WHAT
  boundary the mock replaces, and HOW the mock is verified faithful.

## Process

1. List the external dependencies from the codebase (the integration points).
2. Take the next `IT-XXX`; get the contract with `quoin write . --types IT`.
3. Fill the template: Target Integration (service + protocol), Preconditions
   (environment + data state), Test Data (`TD-XXX` fixtures with rationale),
   Execution Plan (atomic steps, each with a timeout and `IT-XXX-SC-XX` success
   criterion), Expected Outcome (success condition + failure modes).
4. **Traceability is required**: every IT links to ≥1 FR/StR/NFR via frontmatter
   `relationships:` (`type: verifies`). Cross-component references are fine
   (e.g. `ix://org/other-service/FR-001`) per ISO 29148 bidirectional traceability.

## Patterns by service type

- **API (FastAPI/REST)**: CRUD to real endpoints (verify codes/body/DB state);
  invalid payloads → real 4xx; auth with/without creds → real 401/403.
- **Database**: real Alembic migrations → schema state; insert/query/update/delete
  → row state; constraint violations → real DB errors.
- **Event-driven (FastStream)**: publish to a real topic, consume, verify payload;
  produce an event, verify processing + side effects.
- **Worker**: submit a real job → completion state; slow job → timeout + cleanup.
- **Agent (LLM)**: deterministic scripted LLM via real API → verify tool calls;
  real tool execution → file/process state; real SSE/WS → event sequence.
