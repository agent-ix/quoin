---
id: SR-108
title: "Code review — change-assurance integrity contracts"
type: SpecReview
analysis: code-review
scope: "issue #282; US-017, FR-063..FR-065, PLAN-005, src/change-assurance/, retained-evidence exports, schema assets, and TC-1261..TC-1292"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-005"
    type: reviews
  - target: "ix://agent-ix/quoin/TM-001"
    type: references
---

# Code review — change-assurance integrity contracts

## Summary

Reviewed the complete issue #282 implementation against US-017, FR-063..FR-065,
PLAN-005, the three normative JSON Schemas, ix-flow FR-013/018, and Quoin
FR-030/032. The implementation is a reusable library over retained inputs: it
runs no proof, workflow, audit, Git command, or network request and makes no
authentication, authorization, signature, or identity claim.

The review found seven substantive defects. All were fixed and backed by
executable regressions before this artifact was finalized.

## Verdict

**PASS** — no unresolved code-review, integrity-boundary, traceability,
determinism, or code-test-alignment finding remains.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                                                                                     | Refs                                                             | Escape Cause                    |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------- |
| FND-074 | high     | Fixed: the strict JSON parser assigned `__proto__` into a normal object, allowing inherited fields to evade closed-schema validation. Parsed objects now have a null prototype and validators require own fields and a plain/null prototype.                                                | FR-063-AC-7; TC-1267                                             | correct-requirement-no-evidence |
| FND-075 | high     | Fixed: review validation trusted a caller-supplied `chain_valid` boolean. It now consumes exact retained ix-flow event shapes and independently reproduces the FR-013 SHA-256 hash chain before matching a human decision.                                                                  | FR-063-AC-10; FR-065-AC-7; TC-1270; TC-1287                      | correct-requirement-no-evidence |
| FND-076 | high     | Fixed: same-identity attestations and audits were collapsed through last-wins maps, so input order could change a receipt or suppress an unhealthy report. Duplicate identities now fail closed and both permutations produce identical receipts.                                           | FR-065-AC-3; FR-065-AC-10; TC-1283; TC-1290                      | correct-requirement-no-evidence |
| FND-077 | high     | Fixed: the three normative public schema `$id`s existed only in specification fences. Exact versioned JSON assets are now packaged under `dist/schemas`, exported through both required seams, and checked byte-for-byte against the specifications and runtime values.                     | FR-063-AC-1; FR-064-AC-1; FR-065-AC-1; TASK-025                  | correct-requirement-no-evidence |
| FND-078 | high     | Fixed: malformed retained records or attestations could be dereferenced after validation failed. Record schema failures now produce an invalid `schema_invalid` receipt; malformed selected attestations and retained FR-032 reports fail closed without throwing through untrusted fields. | FR-065-AC-1; FR-065-AC-2; TC-1281; TC-1282                       | correct-requirement-no-evidence |
| FND-079 | high     | Fixed: record validation allowed revision 1 with a parent and successors without one, leaving the invariant to a later lineage call. Sealing and verification now enforce the revision/parent invariant and canonical collection order at the record boundary.                              | FR-063-AC-2; FR-063-AC-8; TC-1262; TC-1268                       | correct-requirement-no-evidence |
| FND-080 | medium   | Fixed: command paths accepted non-normalized repository-relative forms, timestamps accepted non-RFC3339 strings, and durable writes ignored short writes/directory durability. Validation and persistence now enforce the declared boundaries.                                              | FR-063-AC-3; FR-064-AC-1; FR-064-AC-6; TC-1263; TC-1272; TC-1277 | correct-requirement-no-evidence |

## Method

- Traced every FR-063..FR-065 acceptance criterion through PLAN-005, the matrix,
  runtime code, public schema assets, and direct `TC-1261..TC-1292` tags.
- Compared the retained event adapter and hash algorithm to ix-flow's public
  FR-013 event contract; no ix-flow command or runtime workflow is invoked.
- Compared the audit adapter to the existing FR-032 `AuditReport`; findings and
  healthy/unevaluated states are consumed unchanged rather than recomputed.
- Exercised malformed raw JSON, prototype keys, digest mutations, missing and
  duplicate inputs, broken event chains, binding mismatches, failed and
  incomplete evidence, crash seams, and equivalent-input permutations.
- Confirmed the new storage family remains separate from FR-030 and that public
  exports preserve existing command behavior.

## Validation evidence

- Full Quire-backed repository gate: 68 test files and **839/839 tests passed**.
- Focused change-assurance and schema-asset suite: **37/37 tests passed**.
- Type checking, ESLint, Prettier, Vite/declaration build, schema packaging,
  version agreement, diff checking, and Quire artifact validation pass.
- The target matrix reports FR-063..FR-065 and TC-1261..TC-1292 covered; the
  mechanical gap analysis is recorded separately as SR-109.

## Boundary

This review covers retained-data integrity and deterministic verification only.
Hashes and recorded actor labels remain integrity and attribution values, not
identity or authority. Promotion, pushing, PR creation, and merge are outside
this local review.
