---
id: SR-110
title: "Code review of governed graph adapters and portfolio"
type: SpecReview
analysis: code-review
scope: "PLAN-006; StR-007; FR-066..FR-067; Quoin #152 at 17ed860; quire-rs assurance-v1 at 3fe2c7e"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-006"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-066"
    type: references
  - target: "ix://agent-ix/quoin/FR-067"
    type: references
---

# Code review of governed graph adapters and portfolio

## Summary

The review covered exact producer-contract validation, canonical identities,
raw scorer retention, plan gating, normalized partitions, tolerant collection
reads, current/history/comparison selection, explicit graph-input mappings,
#152 analyzer injection, human/JSON rendering, command wiring, test quality,
mock boundaries, and reverse code-to-spec ownership.

## Verdict

**PASS.** Two filesystem-boundary findings were corrected and regression-tested.
No open Golden Path, mock-boundary, completeness, code-test alignment, or
reverse-traceability defect remains in the reviewed change.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                                                                                                            | Refs                                                |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------- |
| FND-001 | high     | Resolved: a malformed collection returned an error without a graph availability tag, so the governed filesystem wrapper classified it as `unknown`. The wrapper now normalizes each per-file read failure to `unreadable`, and TC-1312 exercises the real boundary while preserving the valid sibling.             | FR-067-AC-8; TC-1312                                |
| FND-002 | high     | Resolved: tolerant read results are filename-ordered, but FR-045 latest/comparison semantics are timestamp then collection-id ordered. The single-read snapshot now restores the canonical collection order before building the inherited portfolio fields; TC-1312 uses anti-correlated filenames and timestamps. | FR-067-AC-2, AC-8, AC-10; TC-1306, TC-1312, TC-1314 |

## Review evidence

- Tests execute real zod/AJV validation, canonical hashing, measurement
  validation, filesystem reads, the #152 input loader and analyzers, and the
  report command. The only mock is `console.log`, the command output boundary,
  and `afterEach` restores it.
- No `TODO`, `FIXME`, skipped/only test, unsafe double cast, producer process,
  Quire subprocess, Git/network call, graph traversal, or evidence write occurs
  in the pure adapter and portfolio surfaces.
- Quire intake validates the pinned assurance-v1 JSON Schema before the local
  typed handoff, then requires the accepted source and module/schema premises.
- Graph-quality intake verifies the canonical record id, exact scorer bytes,
  full invocation attestation, active plan identity/definition, and a bijective
  partition mapping before a measurement collection exists.
- Mapping conflicts are resolved before reads. Partial export/premises/audit
  triples remain local `incompatible` gaps, and accepted triples flow through
  #152's `loadGraphAnalysisInput` plus exact analyzer functions.
- The portfolio core treats #152 report objects as opaque, performs no relation
  traversal, preserves partitions and raw identities, and gates deltas on every
  declared comparison premise.
- The tolerant filesystem seam reads each collection store once, retains valid
  siblings, classifies malformed siblings locally, and restores the legacy
  timestamp/id order before deriving FR-045 latest and comparison fields.
- Existing non-graph report behavior stays on the original strict path; owner
  and action are loaded only for the governed view, so legacy JSON does not
  acquire new fields.
- Reverse inspection maps every changed production file to FR-066 or FR-067.
  The inherited graph contract remains owned by FR-062/#152; #281 defines no
  second public graph-report model and reparses no specifications.

## Validation

- Combined #152/#281 seam and regression slice: 58/58 pass.
- Code-review regression slice: 18/18 pass.
- `make lint`: pass (TypeScript, ESLint, Prettier).
- `corepack pnpm run build`: pass, including declaration generation and schema copy.
- Full Vitest suite with the repository-pinned local Quire 0.30.2 binary:
  845/845 pass. The shell-selected Quire 0.31.0 is intentionally not the
  governed contract and reproduces the existing TC-118 contract-drift guard.
- The outer `make test` verification-stack wrapper was not used because this
  issue's explicit boundary forbids entering `filament-ide-rs`; the direct
  build, full suite, focused seams, lint, and Quire validations cover #281.
