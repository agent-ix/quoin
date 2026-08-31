---
id: PLAN-005
title: "Change-assurance integrity contracts"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/quoin/US-017"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-063"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-064"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-065"
    type: "references"
---

# PLAN-005: Change-assurance integrity contracts

## Objective

Deliver Quoin #282 as reusable, engine-independent record, attestation, and
verification contracts over retained evidence. Preserve exact integrity and
three-state outcomes without running proofs or making identity claims.

## Requirements Summary

### User requirement

- [x] **US-017:** Reviewers can verify candidate evidence against the exact
      approved definition without treating hashes as signatures.

### Functional requirements

- [x] **FR-063:** Parse, canonicalize, seal, retain, and verify revisioned
      `ChangeAssuranceRecord` values.
- [x] **FR-064:** Validate and atomically retain producer `ProofAttestation`
      values with exact output bytes.
- [x] **FR-065:** Derive deterministic valid/invalid/incomplete receipts from
      retained records, events, attestations, output, and audit findings.

## Dependency Graph

- `FR-063 canonical integrity -> FR-064, FR-065`
  Reason: all three record families share the raw JSON, RFC 8785, BLAKE3, digest,
  and identity primitives.
- `FR-063 lineage/event validation + FR-064 attestation intake -> FR-065`
  Reason: receipt checks consume retained parents, exact workflow decisions,
  attestations, and output bytes.
- `FR-030 -> FR-063, FR-064` and `FR-032 -> FR-065`
  Reason: this lane extends existing retained-evidence storage and consumes the
  existing auditor verdict without replacing it.
- `ix-flow FR-013/018 -> FR-063, FR-065`
  Reason: ix-flow owns event hashes and history; Quoin receives and verifies
  retained event data only.

### Shared dependencies

One strict raw-JSON/JCS/BLAKE3 module owns parsing, canonical bytes, digest
verification, closed schema validation, and deterministic ordering. No existing
generic sorted-JSON helper may substitute for RFC 8785.

### The seams

The new library lives beside `src/evidence/` and may reuse its repository-root
and safe-path conventions, but uses a distinct versioned store family. Public
exports flow through `src/evidence/index.ts` and `src/index.ts`; FR-032 inputs
are adapted from existing auditor types rather than re-evaluated.

## Test Plan

### FR-063 — records and canonical integrity

- [x] **TC-1261 / FR-063-AC-1:** Closed schema requires every record field.
- [x] **TC-1262 / FR-063-AC-2:** Collection properties preserve empties and reject duplicate identities.
- [x] **TC-1263 / FR-063-AC-3:** Requirements and proofs retain exact reviewed premises.
- [x] **TC-1264 / FR-063-AC-4:** Incomplete impact and unknown states stay visible.
- [x] **TC-1265 / FR-063-AC-5:** RFC 8785 vectors and pinned BLAKE3 digests agree.
- [x] **TC-1266 / FR-063-AC-6:** Mutating every semantic leaf invalidates the digest.
- [x] **TC-1267 / FR-063-AC-7:** Malformed raw JSON, Unicode, numbers, ordering, and digest encodings fail closed.
- [x] **TC-1268 / FR-063-AC-8:** Genesis and strict N-1 lineage rules hold under generated chains.
- [x] **TC-1269 / FR-063-AC-9:** Successors preserve exact parent paths and bytes.
- [x] **TC-1270 / FR-063-AC-10:** Only one exact integrity-valid human decision binds.
- [x] **TC-1271 / FR-063-AC-11:** Static boundaries prohibit execution and identity claims.

### FR-064 — attestation intake

- [x] **TC-1272 / FR-064-AC-1:** Closed schema requires every attestation field.
- [x] **TC-1273 / FR-064-AC-2:** Four producer results remain distinct facts.
- [x] **TC-1274 / FR-064-AC-3:** Exact output digest/size mismatch leaves no artifacts.
- [x] **TC-1275 / FR-064-AC-4:** JCS/BLAKE3 vectors and semantic mutations verify.
- [x] **TC-1276 / FR-064-AC-5:** Missing fields are independently refused.
- [x] **TC-1277 / FR-064-AC-6:** Pair persistence is idempotent, crash-atomic, recoverable, and collision-safe.
- [x] **TC-1278 / FR-064-AC-7:** Unavailable diagnostics persist while absence remains absent.
- [x] **TC-1279 / FR-064-AC-8:** Existing evidence records remain byte-compatible.
- [x] **TC-1280 / FR-064-AC-9:** Static boundaries prohibit execution and verdict claims.

### FR-065 — verification receipts

- [x] **TC-1281 / FR-065-AC-1:** Closed receipt schema retains every required input and result.
- [x] **TC-1282 / FR-065-AC-2:** Invalid dominates incomplete, which dominates valid.
- [x] **TC-1283 / FR-065-AC-3:** Selection multiplicity, unknown ids, ordering, and non-selection semantics hold.
- [x] **TC-1284 / FR-065-AC-4:** Every record/candidate/proof/command/tool/config mismatch is named.
- [x] **TC-1285 / FR-065-AC-5:** Only passed exact output with healthy obligations discharges.
- [x] **TC-1286 / FR-065-AC-6:** Missing/unavailable/not-computed/not-evaluated stays incomplete.
- [x] **TC-1287 / FR-065-AC-7:** Event history, uniqueness, decisions, and integrity classify correctly.
- [x] **TC-1288 / FR-065-AC-8:** Impact gaps and unresolved unknowns force incomplete.
- [x] **TC-1289 / FR-065-AC-9:** Exact FR-032 finding kinds and obligation ids survive mapping.
- [x] **TC-1290 / FR-065-AC-10:** Input permutations yield byte-identical receipts and digests.
- [x] **TC-1291 / FR-065-AC-11:** Existing outputs remain byte-identical and verification stays read-only.
- [x] **TC-1292 / FR-065-AC-12:** Static/golden evidence excludes identity and signature claims.

## Remaining Work

### Track A: Critical Path (serial)

- **A1 = TASK-025** Strict canonical integrity core — Hard; exit: official JCS vectors, raw malformed inputs, and semantic mutations pass.
- **A2 = TASK-026** Record lineage and decision validation — Hard; exit: retained chains and exact human decisions classify without side effects.
- **A3 = TASK-029** Receipt integrations — Hard; exit: ix-flow and FR-032 retained inputs produce exact three-state proof/review checks.
- **A4 = TASK-030** Determinism and compatibility — Medium; exit: permutations are byte-identical and prior evidence/report goldens do not move.
- **Gate = TASK-031** Full assurance gate — measures traceability and nondisruption; pass: 32/32 rows backed, repository gates and reviews green.

### Track B: Parallel after canonical core

- **B1 = TASK-027** Atomic attestation intake — Hard; exit: paired artifacts are crash-atomic, idempotent, and collision-safe.

### Track C: Post-attestation

- **C1 = TASK-028** Pure receipt evaluator — Hard; exit: all selection, binding, precedence, and evidence-state properties pass without I/O.

## Parallel Execution Summary

```text
TASK-025 ──> TASK-026 ───────────────> TASK-029 ─> TASK-030 ─> TASK-031
    └──────> TASK-027 ─> TASK-028 ────────┘
```

## Task File Mapping

| Task     | Track | Owns (references) | Verified by (verifies) | Status      |
| -------- | ----- | ----------------- | ---------------------- | ----------- |
| TASK-025 | A     | FR-063            | TC-1261..TC-1267       | not_started |
| TASK-026 | A     | FR-063            | TC-1268..TC-1271       | not_started |
| TASK-027 | B     | FR-064            | TC-1272..TC-1280       | not_started |
| TASK-028 | C     | FR-065            | TC-1281..TC-1286       | not_started |
| TASK-029 | A     | FR-065            | TC-1287..TC-1289       | not_started |
| TASK-030 | A     | FR-065            | TC-1290..TC-1292       | not_started |
| TASK-031 | Gate  | FR-063..FR-065    | TC-1261..TC-1292       | not_started |

## Coordination Rules

- Freeze the schemas and canonical byte contract after TASK-025; downstream
  work imports them rather than copying shapes.
- Keep persistence single-writer: TASK-027 owns the new store paths and atomic
  directory protocol; TASK-026 owns record lineage paths.
- Do not wire external commands, Git, networks, proof execution, or identity
  providers. All adapters accept retained data.
- Merge pure evaluator work only after the attestation type and selection input
  are stable; run compatibility goldens after all code paths meet.
