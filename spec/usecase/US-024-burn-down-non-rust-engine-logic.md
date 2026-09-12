---
id: US-024
title: "Retire Quoin's non-Rust engine logic without losing behaviour or evidence"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
---

# US-024: Retire Quoin's non-Rust engine logic without losing behaviour or evidence

## Story

**As an** owner accountable for the campaign's implementation-language boundary
**I want** Quoin's first-party engine logic to move to Rust one capability at a time, each move
reversible and each proven against the implementation it replaces
**So that** I can watch the non-Rust surface fall on a measurable slope without ever discovering,
after the fact, that a digest moved, a refusal disappeared, or a green check ran over nothing.

The story states what the owner needs in order to authorise each cutover. It does not say which
boundary mechanism is used, how the crates are divided, or what the report looks like; those belong
to the requirements it drives.

## Context

`engineering-assurance` completed this transition and its record is the reference: ADR-002,
StR-003, FR-014 through FR-019, NFR-005, all Python lanes retired. Quoin is the same shape and
several times the size — 105,814 physical lines across 364 in-repository files, measured at
`e718d45`. The largest block is `skills/` at 23,164 lines, which nobody had counted, and which is
not inert: `src/flows.ts:57` spawns ix-flow against `skills/*/workflow-assets/**` and their
`specInvariants` decide whether a review, matrix or plan flow passes. The `corpus/` tree is a
submodule pointing at another repository, `agent-ix/qa-corpus`.

Three facts shape what the owner is asking for. Quoin's digests are *identifiers* inside retained
records, so a digest that changes is not a regression to fix later, it is an unrecoverable one.
Quoin's 109 TypeScript test files are the only oracle that says what the current behaviour is, so
deleting them early destroys the evidence the port needs. And a previous measurement in this
ecosystem reported a check as clean when the check had never executed, which is why the owner will
not accept a removal justified by a check whose population was empty.

A parallel programme already exists: quire-research EPIC #56 contains the spread. This story is the
burn-down, not the containment. Neither may report the other's work as its own.

## Acceptance Examples (Illustrative)

These examples describe what the owner expects to be able to see. They are illustrative and are not
verification criteria.

### US-024-EX-1: A cutover that goes wrong is undone, not patched forward

- **Given** a capability whose Rust replacement has been dispatched to
- **When** the replacement turns out to be wrong
- **Then** reverting the dispatch restores the retained implementation, and no evidence or corpus
  byte was rewritten in either direction

### US-024-EX-2: Deletion is the last step, never the first

- **Given** a capability whose Rust replacement passes
- **When** the owner looks at how it landed
- **Then** the cutover and the deletion are separate changes, and the deletion removes the code and
  its tests together

### US-024-EX-3: A check that ran over nothing is not evidence

- **Given** a removal justified by a check that found no violations
- **When** the owner asks what population it ran over
- **Then** either the population is named and non-empty, or a planted violation is shown to fail the
  check, and otherwise the removal is not accepted

### US-024-EX-4: Coexistence is visible as debt, not as progress

- **Given** a capability whose TypeScript is retained while its Rust replacement is built
- **When** the owner reads the programme's report
- **Then** that path appears as retained-with-successor, naming the ticket that will retire it and
  the date it expires, and never as remediated

### US-024-EX-5: The approved TypeScript is not counted as debt

- **Given** the user-interface code and the types `filament-core-data` publishes
- **When** the owner reads the metric
- **Then** those lines appear as allowed, and no ticket in this programme proposes removing them

## Options (Exploratory)

Approaches raised in discovery, none of which imply commitment: a subprocess boundary carrying JSON;
a native Node addon; a WebAssembly module; a separate Rust repository. Discovery observed that the
repository already shells out to a Rust binary through `src/quire/exec.ts`, and that the evidence
and measurement stores perform filesystem work that a browser-shaped target would have to restore.

## Constraints (Contextual)

Discovery observed that no evidence byte or accepted-corpus byte may change; that the command
surface must stay stable until it deliberately changes; that user-interface TypeScript and
`filament-core-data`-published types are approved and out of scope for removal; and that nothing
publishes to crates.io or public npmjs while the work runs. These are context for the requirements,
not binding statements.

## Dependencies (Contextual)

Upstream: the amended implementation language policy, and the `filament-core-data` schema gate that
decides when a hand-written type may be retired. Downstream: the enforcement scanner that publishes
the metric, and the containment programme whose matrix this burn-down consumes rather than
duplicates.

## Priority and Risk (Informative)

Business value is high: the campaign's language boundary cannot be asserted anywhere while its
largest first-party non-Rust surface is exempt. The risk if unmet is a permanent exemption. The risk
if done badly is worse than not doing it — a port that silently changes an identity domain
invalidates every record that cites it.

## Notes (Informative)

Open question raised in discovery and deliberately left open: whether the published oclif extension
contract — the `plugins` array and the `command_not_found` hook — is ported, kept behind an escape
hatch, or dropped. Captured for the final stage; it introduces no requirement here beyond continuity
until that decision is taken.

## Traceability (Informative)

This story is the Quoin sibling of the Engineering Assurance port recorded in
`ix://agent-ix/engineering-assurance/StR-003`. It traces to
[StR-009](../stakeholder/StR-009-one-implementation-language-for-engine-logic.md) and is expected to
drive [FR-096](../functional/FR-096-versioned-rust-engine-boundary.md) through
[FR-103](../functional/FR-103-corpus-consolidation.md).
