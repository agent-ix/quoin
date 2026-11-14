# Requirements Review Checklist

## ID Format and Uniqueness
- [ ] All user stories use `US-XXX` format (3-digit).
- [ ] All functional requirements use `FR-XXX` format.
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

## Criterion Coverage Quality
- [ ] Coverage: `quire matrix` shows no `untagged` criterion — every AC is carried by >= 1 test's trace tag.
- [ ] Option Permutation: All valid combinations have a criterion.
- [ ] Constraint Boundary: All boundaries have a criterion (min, max, over, under).
- [ ] Error Path: All error conditions have a criterion.
- [ ] State Transition: All transitions have a criterion (if applicable).
- [ ] Edge Case: Edge cases identified and stated as criteria.
- [ ] `Verification` cells name a method only, with no `TC-` id.

## Hash / Digest / Pin Antipattern (any hit is a `high` finding except the narrow identity-digest case)

**Mandatory in base, all, and subset reviews.** Mirror `/code-review`'s P0 rule
for unasked-for ceremony and file tracking: we have git; do not specify a second
file-tracking system. Review requirements, acceptance criteria, test cases, ADRs,
and implementation instructions before they become code.

Search for hash, digest, SHA, checksum, pin, manifest, ledger, provenance,
attestation, receipt, relocation, inventory, and recorded/reviewed revision uses;
a keyword alone is not a finding, but a requirement that instructs any of them is,
except for the identity-digest case below. For every violation, record a P0
finding with `Severity: high`, a FAIL verdict in the review summary, the exact
artifact/criterion references, and removal as the required correction. A valid
review document does not make a failing spec ready for implementation.

The sole exception is an **identity digest**: a canonical digest supplied or
verified only to bind a signed statement or proof to the exact content it
attests, when that identity binding is itself the feature. It must not serve as
a ledger, manifest, pin, version record, file-tracking index, or provenance
record. For example, it is allowed to verify that a signed in-toto statement's
subject `sha256` matches the exact artifact bytes it attests. It is not allowed
to store hashes for repository files in a manifest and compare them later to
track changes; that remains file tracking.

- [ ] No requirement, criterion, test case, ADR or instruction creates, keeps or checks a hash, digest, SHA, checksum, pin, checksum catalog, manifest, ledger, inventory, provenance record, attestation, receipt, relocation map, ID-block table or recorded/reviewed revision, except for the identity-digest case above.
- [ ] No test case counts or scans files, fixtures, versions, spec text or source text. Hashing exact content is allowed only for the identity-digest case above.
- [ ] No exact tool or toolchain version pinned in prose. Package versions live in `Cargo.toml` / `package.json` and their lockfiles only; a report may name the app version it ran.
- [ ] Existing use, another repo's format, or "a test needs it" is not a justification. No exception applies beyond the identity-digest case defined above; do not propose an "informational" carve-out or a replacement check.

## Cross-Referencing
- [ ] FR links to US.
- [ ] Test trace tags name AC ids, not `TC-` ids.
- [ ] Full IDs used.
- [ ] Links valid.
- [ ] Terminology consistent.
