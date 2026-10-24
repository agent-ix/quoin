# Deriving Functional Requirements from a User Story

Load this when a request involves turning user stories into functional
requirements. It carries the *reasoning* for the derivation; the per-FR section
rules come from the `FR` template (`quoin write . --types FR`).

## Rules

- **Atomic**: decompose the story into specific, single-behavior FRs.
- **Normative**: FRs use `SHALL` / `SHALL NOT` (a US never does).
- **No copying**: translate intent into a technical specification — do not paste
  the story text.
- **Verifiable**: every FR carries its own Acceptance Criteria table.

## Process

1. **Analyze** the `US-XXX`: list the system responsibilities it implies.
2. **Decompose** into typed FRs. A behavioral FR (no `object:`) captures logic
   and business rules. A domain-typed FR carries `object: <type>` and is purely
   structural:
   - `entity` — data model with typed fields
   - `event` — a domain event emitted
   - `api_endpoint` — a REST/HTTP endpoint
   - `queue` — a message contract
   - `process` — an end-to-end workflow
   - `state_machine` — an entity lifecycle
   - `configuration` — a configuration schema
   - Full list + relationship verbs: `spec-object-review`'s
     `references/object-type-guide.md`; inspect any type with
     `quoin catalog show <type>`.
3. **Author** each FR as its own file (`quoin write . --types FR` for the contract),
   and link it back to the story via frontmatter `relationships:`
   (`type: implements`, target the `US-XXX`).
4. **Specify** inputs/outputs (typed and validated), error handling (failure
   modes), and constraints (technical limits, not options).
5. **Separation of concerns**: keep behavioral triggers/validation in behavioral
   FRs; domain-typed FR acceptance criteria test **structure** only (schema/field
   presence), never behavior. Do not duplicate behavioral ACs into a domain FR.
6. **Save** to `spec/functional/FR-XXX-description.md` (single repo) or
   `specs/<category>/<component>/spec/functional/FR-XXX-description.md` (multi-repo).
