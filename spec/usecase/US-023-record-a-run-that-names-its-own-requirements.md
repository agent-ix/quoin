---
id: US-023
title: "Record a test run whose result file names the requirements it verified"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
---

# US-023: Record a test run whose result file names the requirements it verified

## Story

**As a** test author who tags tests with requirement trace ids
**I want** the result file of an ordinary suite run to carry those ids on each test case
**So that** recording the run binds the obligations that actually passed, and a failing test discharges nothing.

## Context

The author already writes the trace id in source. The suite runs in CI under
pytest, vitest or Rust. Today the result file contains only test names and
outcomes, so nothing connects a passing test to the requirement it was written
for, and the evidence store stays empty.

## Acceptance Examples (Illustrative)

### US-023-EX-1: A passing tagged test binds its obligation

- **Given** a test tagged `FR-001-AC-1` that passes in a run that records evidence
- **When** the author records the result file
- **Then** the obligation for `FR-001-AC-1` is bound to that test and run

### US-023-EX-2: A failing tagged test binds nothing

- **Given** the same test, failing or panicking
- **When** the author records the result file
- **Then** the entry is recorded as failed and the obligation is not discharged

### US-023-EX-3: An ordinary run is unchanged

- **Given** the same suite run without evidence recording enabled
- **When** the tests execute
- **Then** the build and the output are exactly what they were before

## Constraints (Contextual)

The suite must not become measurably slower. The recording step reads the result
file only and never runs the suite.

## Dependencies (Contextual)

Upstream: the source markers (quire-rs, spec-artifacts-process `trace_tags`).
Downstream: FR-120, and the existing evidence store (FR-030) and
auditor (FR-032).

## Priority and Risk (Informative)

Priority is P1: no repository has a populated evidence store. The risk is a
symbol that differs between the run and the static extractor, so the two views
never join.
