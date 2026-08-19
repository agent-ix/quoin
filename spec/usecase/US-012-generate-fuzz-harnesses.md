---
id: US-012
title: "Generate fuzz harnesses for the input surfaces my spec names"
type: US
relationships:
  - target: "ix://agent-ix/quoin/FR-038"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---

# US-012: Generate fuzz harnesses for the input surfaces my spec names

## Story

**As a** developer whose specification says the parser must not crash on arbitrary input
**I want** that requirement to arrive as a fuzz target in my own harness, calling a function
that actually exists
**So that** a robustness requirement stops being a sentence nobody can act on, without a tool
deciding on my behalf that my repository is now a fuzzing repository.

This story expresses the developer's perspective in informal language and avoids prescribing
the harness, the fuzzing engine, or the file layout.

## Context

The verification-method catalog already names fuzzing, and the method advisor already
recommends it for requirements about parsers and untrusted input. Between that recommendation
and a running fuzzer there is nothing: the developer is told what method applies and left to
build it by hand.

Two things make this different from generating a property test.

A fuzz target **calls a symbol**. A generated test that grounds nothing is a weak test; a
generated fuzz target that calls nothing does not compile, and it fails in a harness most
reviewers never run locally.

And a fuzz target that exists **has proved nothing**. Fuzzing is a search, so an unrun target
is a search never started — while looking, in every report and every matrix, exactly like
coverage.

Setting a repository up to fuzz at all is also not a small thing: a dev-dependency, a nightly
toolchain, a CI job that runs for minutes rather than seconds. A developer who wrote
"must not panic" did not thereby ask for any of that.

## Acceptance Examples (Illustrative)

These examples clarify the developer's expectations. They are illustrative only — not test
cases and not verification criteria.

### US-012-EX-1: A named surface becomes a runnable target

- **Given** a repository with a fuzz harness and a requirement about its parser
- **When** the developer runs the generation
- **Then** a target arrives in the repository's own harness, calling a parser function that
  exists in the source, carrying the requirement's id

### US-012-EX-2: A repository without a fuzzer is told, not converted

- **Given** a repository whose specification names input surfaces and which has no fuzz
  tooling
- **When** the developer runs the generation
- **Then** they receive a report saying so, and **no** dependency, toolchain file or fuzz
  workspace is created

### US-012-EX-3: An ungroundable requirement produces a question, not a guess

- **Given** a requirement about a surface with no matching function in the source
- **When** the developer runs the generation
- **Then** they are told which requirement and what was looked for, and no file is written
  for it

### US-012-EX-4: Generated targets are never reported as coverage

- **Given** targets that have been generated but never run
- **When** the developer reads the report
- **Then** it says plainly that these discharge nothing until a run is recorded, and no
  coverage row reads as satisfied because a file exists

## Dependencies

- **Upstream**: [StR-004](../stakeholder/StR-004-governed-workflows.md)
- **Downstream**: [FR-038](../functional/FR-038-generate-fuzz-harnesses.md)
