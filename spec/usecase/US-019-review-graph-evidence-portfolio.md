---
id: US-019
title: "Review governed graph evidence across repositories"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---
# US-019: Review governed graph evidence across repositories

## Story

**As an** assurance owner responsible for several repositories
**I want** graph quality, graph structure, history, provenance, and gaps shown under their governing plans and populations
**So that** I can compare compatible evidence without averaging unlike graphs or overlooking unavailable inputs.

## Context

Quire can export the source-grounded artifact graph and quire-code-rs can emit a
versioned quality observation over a pinned truth corpus. Quoin already stores
plan-governed numeric observations and renders repository portfolios. The
missing boundary is a lossless adapter for each producer and a portfolio view
that preserves graph partitions, raw evidence, availability, and comparison
premises.

## Acceptance Examples (Illustrative)

### US-019-EX-1: A governed observation enters once

- **Given** a schema-valid graph-quality observation, matching active plan, raw scorer bytes, and complete invocation attestation
- **When** the owner records it through the graph-quality adapter
- **Then** one atomic collection retains every producer field and normalized partition without running the producer

### US-019-EX-2: Unlike graphs are not compared

- **Given** two collections with different definition versions or population identities
- **When** the portfolio renders their history
- **Then** both remain readable and the comparison is explicitly incompatible with the differing premises named

### US-019-EX-3: Missing graph input is not an empty graph

- **Given** a repository with no supplied Quire assurance export
- **When** graph views are requested in the portfolio
- **Then** graph availability is `missing`, not a zero-node or complete result

## Constraints (Contextual)

Quoin transcribes and reports retained artifacts. It runs no extractor, scorer,
Quire command, or repository producer and computes no cross-repository score.

## Dependencies (Contextual)

Upstream: [FR-044](../functional/FR-044-plan-governed-measurements.md),
[FR-045](../functional/FR-045-portfolio-measurement-report.md), and
[FR-062](../functional/FR-062-read-only-evidence-graph-analysis.md). Producer
contracts are `agent-ix/quire-rs` FR-067/068 and `agent-ix/quire-code-rs`
FR-011/012.

## Priority and Risk (Informative)

Priority is P0. A lossy adapter or a comparison across unlike populations can
turn precise producer evidence into a plausible but unsupported portfolio
claim.
