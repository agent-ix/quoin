---
id: SR-056
title: "Code review — default-module semantic type-fit audit"
type: SpecReview
analysis: code-review
scope: "issue #288; US-014, FR-051..FR-055, NFR-015..NFR-016, PLAN-003, audit implementation and retained projections"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: reviews
  - target: "ix://agent-ix/quoin/TM-001"
    type: references
---

# Code review — default-module semantic type-fit audit

## Summary

Reviewed the complete issue #288 implementation, tests, specifications, plan, and generated artifact
contract against US-014, FR-051..FR-055, NFR-015..NFR-016, and TC-1156..TC-1194. The implementation
is confined to read-only audit code, tests, specifications, plans, reviews, generated analysis, and
the exact formatter exclusion required to preserve content-addressed evidence. It changes no Quoin
runtime source, module declaration, schema, skeleton, registry, generated package, persistence,
consumer contract, publication, or enforcement behavior.

The review found five substantive defects. All were fixed and covered by executable tests before this
artifact was finalized.

## Verdict

**PASS** — no unresolved code-review, safety-boundary, traceability, canonical-artifact, or
code-test-alignment finding remains. Promotion remains independently gated by PR #311's named
maintainer approval and TC-1194.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                                                                                                                                                            | Refs                                               |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------- |
| FND-066 | high     | Fixed: 38 passing audit tests were declared through a custom `tc(...)` wrapper that Quire could not recognize, making every automated issue #288 matrix row a status lie. The tests now use direct `it("TC-…")` declarations; targeted Quire coverage reports zero unbacked rows, status lies, or untracked symbols.                                               | FR-051..FR-055; NFR-015..NFR-016; TC-1156..TC-1193 |
| FND-067 | high     | Fixed: the audit could report `clean` whenever provenance and denominators reconciled even if declarations were incomplete or required concepts were absent/overloaded. Verdict derivation now requires acceptable per-type dispositions and represented concepts, with a regression test.                                                                         | FR-053-AC-3; FR-053-AC-8; TC-1171; TC-1176         |
| FND-068 | high     | Fixed: a pre-existing symlink at a canonical output filename could redirect `writeFileSync` outside the configured output directory. The writer now rejects a non-directory output root and every non-regular pre-existing artifact target; TC-1192 proves the redirect target remains unchanged.                                                                  | NFR-016; TC-1192                                   |
| FND-069 | medium   | Fixed: artifact verification checked counts, digests, and finding-id presence but did not prove one-to-one manifest paths/schema versions or exact Markdown derivation. The manifest now versions every artifact, rejects duplicate/missing paths, binds the run timestamp, recomputes the semantic summary, and byte-compares both projections to canonical JSON. | FR-054-AC-4; FR-054-AC-5; TC-1180; TC-1181         |
| FND-070 | medium   | Fixed: a single Git blob read failure could abort collection of every sibling Markdown file. Corpus reads now retain an `io-error` document row and acquisition finding per failed path while continuing the census; declaration evidence also uses the unique declaration id rather than an ambiguous qualified name.                                             | FR-052-AC-2; FR-052-AC-4; TC-1163; TC-1165         |

## Method

- Reviewed all issue #288 code paths for denominator self-reconciliation, failure isolation, unsafe
  path handling, symlink traversal, shell interpolation, silent catches, stale projections, mutable
  identities, weak assertions, skipped tests, mocks, placeholders, and requirement weakening.
- Reconstructed the canonical artifact family from the retained JSON inputs and required exact
  generated report and SpecReview bytes, rather than accepting independently edited projections.
- Reconciled every TC-1156..TC-1193 assertion to its owning acceptance criterion and ran Quire's
  reverse symbol analysis. The final target slice has zero unbacked rows, status lies, and untracked
  symbols. TC-1194 remains manual by design.
- Confirmed subprocess calls use argument arrays rather than a shell, output filenames are fixed,
  all source/module reads are read-only, and the changed-path guard rejects runtime, schema,
  manifest, generated-package, migration, and consumer changes.
- Confirmed every new JavaScript source file carries `AGPL-3.0-or-later` and the repository remains
  AGPL-3.0-or-later.

## Validation evidence

- Focused architecture and audit contracts: 69/69 passed, including 38/38 TC-1156..TC-1193 cases.
- Targeted Quire reverse traceability: 0 unbacked rows, 0 status lies, 0 untracked symbols.
- Type checking, ESLint, Prettier, build, diff checking, and Quire artifact validation pass.
- The full inherited repository suite currently passes 800/802. The two failures are external-state
  drift also present outside this diff: the selected Quire engine emits a coverage field newer than
  Quoin's vendored contract, and `tests/skill-contracts.test.ts` reads the mutable local
  `spec-artifacts-process` checkout whose manifest predates `architecture-evaluation`. This branch
  changes neither contract surface; the exact limitation remains visible rather than being bypassed.

## Promotion boundary

The audit identifies future compiler, code-generation, module-schema, migration, database, API,
publication, enforcement, and retirement work but activates none of it. A stacked PR may be opened;
it must remain unmerged until the normative architecture in PR #311 has named maintainer approval.
