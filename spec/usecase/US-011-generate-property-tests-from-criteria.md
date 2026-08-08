---
id: US-011
title: "Generate property tests from acceptance criteria"
type: US
relationships:
  - target: "ix://agent-ix/quoin/FR-028"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---

# US-011: Generate property tests from acceptance criteria

## Story

**As a** developer whose acceptance criteria are already classified by property shape
**I want** those criteria turned into runnable property tests, with the ones the
classifier could not settle routed to me for review rather than guessed at
**So that** the criteria I already wrote carry their own verification, and I spend my
attention only on the residue a deterministic pass cannot reach.

This story expresses the developer's perspective in informal language and avoids
prescribing the harness, the generator library, or the file layout.

## Context

The classification of a criterion by property shape is available as data, per
criterion, keyed on the criterion's row id. It stops deliberately short of a test:
it names no framework, and its deterministic recall has a measured ceiling, so a
sizeable share of criteria arrive labelled as examples or unclassified rather than
as properties.

Developers do not want a report about that residue; they want tests. They also do
not want to be told their criteria scored badly — a criterion that describes a
single concrete scenario is a legitimate criterion, not a defect. So the generated
output must separate what a machine settled from what a person still has to look
at, and it must never read as pressure to reword a specification to please a
checker.

## Acceptance Examples (Illustrative)

These examples clarify the developer's expectations. They are illustrative only —
not test cases and not verification criteria.

### US-011-EX-1: Settled criteria become runnable tests

- **Given** a repository whose criteria include some the classifier settled as
  quantifying over an input domain with a stated oracle
- **When** the developer runs the generation
- **Then** those criteria arrive as property tests in the repository's own test
  framework, each carrying the criterion's row id, and the suite runs green

### US-011-EX-2: Unsettled criteria arrive queued, not merged

- **Given** criteria the classifier labelled as examples, or labelled as properties
  without corroborating the extraction
- **When** the developer runs the generation
- **Then** proposals for those criteria arrive separately and inert — they cannot
  make a coverage row look satisfied until the developer accepts them

### US-011-EX-3: A census is data, never a grade

- **Given** a repository whose criteria are mostly concrete examples
- **When** the developer runs the generation
- **Then** the counts are reported plainly, with no threshold, no pass or fail, and
  no suggestion to rewrite a criterion

### US-011-EX-4: Nothing is invented

- **Given** a criterion whose oracle cannot be traced to the specification or the code
- **When** the developer runs the generation
- **Then** no test is written for it and the reason is recorded, rather than a test
  that asserts something trivially true

## Dependencies

- Criteria are already classified per criterion and keyed on a stable row id
  ([quire-rs FR-052](ix://agent-ix/quire-rs/FR-052)).
- The repository has a test framework of its own; the generation adopts it rather
  than introducing one.
