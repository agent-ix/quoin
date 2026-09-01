---
id: US-017
title: "Verify candidate evidence against an approved change definition"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---

# US-017: Verify candidate evidence against an approved change definition

## Story

**As an** assurance reviewer responsible for accepting a change
**I want** the reviewed definition, candidate proof evidence, and verification
result bound by content identity
**So that** later edits, unrelated runs, and missing evidence remain visible
without treating integrity metadata as a person's signature.

## Context

A definition of success is reviewed before implementation, while proof output
arrives later from candidate-specific tools. Mutable files and passing labels do
not show that the reviewed definition, candidate revision, proof obligation,
command, configuration, and retained output still belong together. The shared
contracts preserve those joins and let Quoin audit them without running a proof
command.

## Acceptance Examples (Illustrative)

### US-017-EX-1: A reviewed definition verifies

- **Given** a sealed record, a matching ix-flow approval event, and one healthy
  selected attestation for every required proof
- **When** Quoin verifies the candidate from the retained inputs
- **Then** the receipt is `valid` and names every input digest

### US-017-EX-2: A later edit is detected

- **Given** a sealed record whose requirement, constraint, proof, source,
  unknown, or parent field is edited without resealing
- **When** Quoin verifies the record
- **Then** the receipt is `invalid` with `record_digest_mismatch`

### US-017-EX-3: Missing evidence stays incomplete

- **Given** an approved record whose required proof has no selected attestation
- **When** Quoin verifies the candidate
- **Then** the proof and receipt are `incomplete`, not passing or numerically zero

## Constraints (Contextual)

Quoin reads retained records, workflow events, attestations, outputs, and audit
findings. It executes no proof command. Digests provide content integrity only;
they establish no identity, authorization, or non-repudiation.

## Dependencies (Contextual)

Upstream: [FR-030](../functional/FR-030-evidence-store.md) and
[FR-032](../functional/FR-032-evidence-auditor.md). The human decision input is
an ix-flow FR-013 integrity-verified event produced by its human-review workflow.

## Priority and Risk (Informative)

Priority is P1. The primary risk is a plausible passing run being joined to the
wrong reviewed definition, candidate revision, proof, command, or configuration.
