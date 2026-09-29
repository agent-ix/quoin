# Requirements Review Checklist

## ID Format and Uniqueness
- [ ] All user stories use `US-XXX` format (3-digit).
- [ ] All functional requirements use `FR-XXX` format.
- [ ] All test cases use `TC-XXX` format.
- [ ] Acceptance criteria use `{PARENT}-AC-N`.
- [ ] Options use `{PARENT}-OPT-{LETTER}`.
- [ ] Constraints use `{PARENT}-CON-N`.
- [ ] No duplicate IDs. IDs are sequential.

## User Story Quality
- [ ] Uses "As a/I want/So that" format.
- [ ] >= 2 acceptance criteria (Given/When/Then).
- [ ] Criteria are testable.
- [ ] Options document trade-offs.
- [ ] Constraints include rationale.
- [ ] Dependencies linked.
- [ ] Priority specified.
- [ ] User value focus (not implementation).

## Functional Requirement Quality
- [ ] Description clear/specific.
- [ ] Related US linked.
- [ ] Inputs defined (type, validation).
- [ ] Outputs defined (type).
- [ ] Behavior detailed.
- [ ] Options document impact.
- [ ] Constraints document rationale/validation.
- [ ] Error conditions documented (codes).
- [ ] Performance targets specific.
- [ ] Security addressed.
- [ ] Criteria verifiable.
- [ ] Dependencies documented.

## Test Coverage Quality (Six Rules)
- [ ] Coverage: Every AC has >= 1 TC.
- [ ] Option Permutation: All valid combinations tested.
- [ ] Constraint Boundary: All boundaries tested (min, max, over, under).
- [ ] Error Path: All error conditions tested.
- [ ] State Transition: All transitions tested (if applicable).
- [ ] Edge Case: Edge cases identified/tested.
- [ ] TC fields complete (Type, Priority, Trace).

## Hash / Digest / Pin Antipattern (any hit is a `high` finding)
- [ ] No requirement, criterion, test case, ADR or instruction creates, keeps or checks a hash, digest, SHA, checksum, pin, checksum catalog, manifest, ledger, inventory, provenance record, attestation, receipt, relocation map, ID-block table or recorded/reviewed revision.
- [ ] No test case counts, hashes or scans files, fixtures, versions, spec text or source text.
- [ ] No exact tool or toolchain version pinned in prose. Package versions live in `Cargo.toml` / `package.json` and their lockfiles only; a report may name the app version it ran.
- [ ] The only digest allowed is a canonical identity digest that binds a proof to the exact content it proved, and only where trust and provenance are the feature.
- [ ] Existing use, another repo's format, or "a test needs it" is not a justification. Do not propose a new exception, an "informational" carve-out, or a replacement check; the finding is the removal.

## Cross-Referencing
- [ ] FR links to US.
- [ ] TC links to AC.
- [ ] Full IDs used.
- [ ] Links valid.
- [ ] Terminology consistent.
