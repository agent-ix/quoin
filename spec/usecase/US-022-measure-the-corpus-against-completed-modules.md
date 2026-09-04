---
id: US-022
title: "Measure the governed corpus against the completed module schemas"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-005"
    type: "traces_to"
---

# US-022: Measure the governed corpus against the completed module schemas

## Story

**As a** campaign owner deciding whether a module contract may start failing builds
**I want** a deterministic, advisory measurement of the whole governed corpus against every completed
module schema, with each failure carrying an owner and a disposition
**So that** I can decide whether those schemas may be promoted from advisory to enforcing without
first discovering, one broken repository at a time, that they cannot be.

The story states what the owner needs in order to make the promotion decision. It does not say which
checks run, how a document is classified, or what the report looks like; those belong to the
requirements it drives.

## Context

Nine module repositories completed their semantic contracts in the current wave, and a tenth
(`engineering-assurance`) is pinned in the catalog. Each of them declares vocabulary, JSON Schemas
and — for the artifact-type modules — Markdown mappings that say how an authored document becomes a
record. None of that has ever been run against the documents the ecosystem actually contains.

Two earlier measurements in this programme published headline numbers that were wrong, in both cases
because a figure was quoted without the population it counted. A third reported a check as clean when
the check had never executed. The owner has said repeatedly that a high failure count is a census
result, not evidence that a rule is too strict.

The corpus is read-only for this work. Repositories such as `config-service` and `quire-rs` are
fixtures: their documents are evidence about the contracts, and rewriting them would destroy exactly
the signal the measurement exists to produce.

## Acceptance Examples (Illustrative)

These examples describe what the owner expects to be able to see. They are illustrative and are not
verification criteria.

### US-022-EX-1: A module with a weak partition cannot hide inside a good total

- **Given** one module whose documents conform far less often than the corpus as a whole
- **When** the owner reads the published report
- **Then** that module's own rate is visible beside the total, and the report says the total is not
  representative of it

### US-022-EX-2: A check that could not run is not reported as a check that passed

- **Given** a repository whose documents cannot be read because of a known tool defect
- **When** the owner reads the published report
- **Then** those documents appear in a state that is neither pass nor fail, naming the defect

### US-022-EX-3: Every failure has somebody's name on it

- **Given** any failing document in the corpus
- **When** the owner opens the partition
- **Then** the failure carries a class, an owner and a disposition, and "unknown" is one of the
  states the owner may see rather than a state the measurement is allowed to guess its way out of

### US-022-EX-4: The measurement can be repeated

- **Given** the pins the report records
- **When** somebody else re-runs the measurement from a clean machine
- **Then** they obtain the same counts

## Options (Exploratory)

Approaches raised in discovery, none of which imply commitment: reusing each module's own reference
mapper as the oracle; driving the modules through an installed Quoin catalog; evaluating the
declared mappings generically inside Quoin. Discovery noted that the catalog route is currently
blocked (agent-ix/quoin#347) and that reference mappers exist for only some modules.

## Constraints (Contextual)

Discovery observed that the measurement must not edit a corpus repository, must not fail a build on
findings, and must finish inside a working session on a developer machine. These are context for the
requirements, not binding statements.

## Dependencies (Contextual)

Upstream: the completed module contracts of this wave, and the semantic-module contract that told the
modules how to declare their schemas and mappings. Downstream: the promotion decision that consumes
this report, and the later normalization campaign that consumes its partition.

## Priority and Risk (Informative)

Business value is high: the promotion gate has no other evidence to consume. The risk if unmet is
that enforcement is promoted blind and breaks the ecosystem's repositories at once. The risk if done
badly is worse than not doing it — a measurement believed to be sound and quietly wrong.

## Notes (Informative)

Open question raised in discovery and deliberately left open: whether repositories outside the
workspace should be pulled in for the census. Captured for the later campaign; it introduces no
requirement here.

## Traceability (Informative)

This story sits beside US-014, which drove the earlier default-module type-fit audit. The two share a
subject and not a population: US-014 inventoried the modules, this story measures the corpus.
