---
id: SR-152
title: "failure-domain review of the EPIC #373 Rust burn-down spec set"
type: SpecReview
analysis: failure-domain
scope: "docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md, spec/stakeholder/StR-009-one-implementation-language-for-engine-logic.md, spec/usecase/US-024-burn-down-non-rust-engine-logic.md, spec/functional/FR-096..FR-103, spec/non-functional/NFR-024..NFR-027, spec/integration/IT-003-filament-core-data-published-types.md"
review_set: all
---

# SR-152: failure-domain review of the EPIC #373 Rust burn-down spec set

## Summary

Failure-domain analysis of the fifteen artefacts specifying the staged Rust port
of Quoin's first-party engine behind the `quoin-core` subprocess boundary,
examining extension points and trust boundaries, entity identity, evaluation
purity and topological robustness. The set is unusually complete on planted-
violation and empty-population defences, but twenty gaps remain: the requirement
dependency graph itself contains a cycle (FR-100 → FR-101 → FR-103 → FR-100);
the population of the strongest gate in the set — digest replay over "every
reachable store" — is never defined; the subprocess model introduces concurrency
and abnormal-termination cases against a store whose digests are identifiers,
and neither is constrained; the executable path, which is the identity key of the
entire burn-down metric, has no stated uniqueness rule and does not survive a
rename; the reversibility the owner is promised has no stated validity window;
and the fixture capture that replaces live oracles after cutover carries no
provenance rule, so parity evidence can become a tautology. No spec artefact was
edited.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                    | Refs                                      |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| FND-001 | high     | The requirement dependency graph contains a cycle: FR-103 declares FR-101 upstream, FR-101 declares FR-100 upstream, and FR-103 declares FR-100 downstream — in a set whose ADR forbids Cargo cycles; ADR-0003 separately says corpus consolidation "runs in parallel and independently", contradicting FR-103's upstream declaration. | FR-100, FR-101, FR-103, ADR-0003          |
| FND-002 | high     | "Every reachable store" / "every store reachable in the ecosystem" is never defined, so the digest-replay gate — the hardest gate in the set — has an unspecified domain; requiring the population be non-empty and named does not fix an enumeration rule the caller chooses.                                                          | FR-098, FR-100, NFR-025                   |
| FND-003 | high     | No crash, signal or concurrency rule for store writes. One spawn per operation makes concurrent `quoin-core` invocations against a single evidence store a first-class case, yet nothing states locking, last-writer semantics, torn-record detection or orphan-temp cleanup; exit 3 or 4 partway through a write leaves the store undefined, and NFR-025 constrains only cutover, revert and deletion. | FR-096, FR-100, NFR-025                   |
| FND-004 | high     | The executable path is the identity key of the whole burn-down metric and no uniqueness rule is stated; a rename or move silently drops a retention row together with its successor reference and expiry date, and FR-103-AC-5 makes classification deliberately move-sensitive, so the slope is neither rename-stable nor ungameable. | FR-101, FR-103, NFR-024, StR-009          |
| FND-005 | high     | The revert guarantee has no validity window. Every cutover is required to be revertible, but deleting an earlier stage's retained path makes a later stage's revert impossible; nothing requires dependent cutovers to be identified or reversibility to be declared lapsed.                                                            | FR-101, US-024, NFR-025                   |
| FND-006 | high     | Captured-fixture provenance is unconstrained. Live non-Rust oracles are replaced by fixtures "captured once from the retained implementation", but the only refusal is for a fixture the test wrote in its own body; nothing forbids recapturing from the Rust implementation after cutover, which turns parity evidence into a tautology. | FR-101, FR-098, US-024                    |
| FND-007 | high     | The second trust boundary inherits none of the first one's hardening: `quoin-core` spawning ix-flow against `skills/**/workflow-assets/**` carries no realpath resolution, no digest pin, no output-size floor, no timeout and no three-way termination taxonomy, though those assets' `specInvariants` decide whether a flow passes. | FR-099, FR-096, FR-101                    |
| FND-008 | high     | An unset `QUOIN_EXPECTED_CORE_SHA256` is unspecified: the refusal fires only when bytes "do not match", so an absent expectation is a vacuous pass — precisely the empty-population defect US-024 exists to refuse. The window between digest check and exec is likewise unaddressed.                                                   | FR-096, US-024                            |
| FND-009 | high     | Forward divergence is unguarded. NFR-025 excludes newly written records from its scope and FR-098's replay covers only digests that already exist, so a canonicalization difference that manifests only in records the Rust implementation newly writes is caught by no gate in this set.                                               | NFR-025, FR-098, FR-100                   |
| FND-010 | high     | FR-098 declares diagnostic message text non-contractual while NFR-025 declares every stored byte immutable; nothing resolves the case where a stored evidence or change-assurance record embeds diagnostic text emitted by a ported capability.                                                                                         | FR-098, NFR-025, FR-100                   |
| FND-011 | medium   | The expiry rule is a time bomb with no grace path: a retention whose expiry passes fails the enforcement run on a calendar boundary with no code change, on every branch including the branches doing the porting, and nothing scopes that failure to the report rather than the gate.                                                  | NFR-024, FR-101                           |
| FND-012 | medium   | The boundary operation namespace has no uniqueness rule. Nothing states that `<domain>.<op>` is unique across crates or that dispatch is a duplicate-free total map, though FR-099 requires exactly that discipline of catalog type declarations.                                                                                       | FR-096, FR-099                            |
| FND-013 | medium   | The allowance manifest has no precedence rule for overlapping path globs and no rule forbidding one path from carrying both a retention row and an allowance entry, yet the enforcement run must classify every path into exactly one of three classes.                                                                                 | NFR-024, FR-101                           |
| FND-014 | medium   | Case-insensitive catalog type resolution creates an identity-collision class the duplicate check does not cover: whether duplicate detection folds case, and under which case-folding and Unicode normalization, is unstated, so two declarations can be distinct to the duplicate gate and identical to the resolver.                   | FR-099                                    |
| FND-015 | medium   | A partially materialized module root is indistinguishable from a complete one: a git or network failure mid-materialize leaves a root that then satisfies "already materialized, perform no git or network access", freezing a broken root. No atomicity or completeness check is required.                                              | FR-099, FR-096                            |
| FND-016 | medium   | NFR-026 bounds the toolchain only from below, though its own rationale — `rustfmt` and `clippy` output is version-dependent — applies equally above the floor, so a gate run on a newer toolchain remains a claim about whichever toolchain was on the path.                                                                             | NFR-026, NFR-027                          |
| FND-017 | medium   | Root escape is addressed as a path property but not as a link property: realpath resolution is required only for the `quoin-core` executable, not for the caller-selected repository and configuration roots, so a symlink inside a permitted root reaches outside it.                                                                   | FR-096, FR-099                            |
| FND-018 | medium   | Producer execution is an unbounded extension point. "Bounded producer execution" is named only as a capability consumed from `engineering-assurance`; no time, output-size or resource bound is stated, and no failure policy is given for a producer that hangs, floods stdout or exits by signal.                                      | FR-100, FR-096                            |
| FND-019 | medium   | The digest-citation graph has no termination or depth bound. Records cite records by digest and the replay enumerates them, but nothing guarantees termination on a cyclic or deeply nested citation chain; `quoin-graph-analysis` likewise carries no termination or worst-case-structure requirement.                                  | FR-098, FR-100, NFR-025                   |
| FND-020 | low      | FR-096's Outputs claims "exactly one versioned JSON result document on stdout per invocation" while its Behavior requires a non-zero status carrying no payload to be distinguishable; the malformed-JSON and unknown-version paths cannot emit a versioned document, so the two statements disagree on the non-success contract.        | FR-096                                    |

## Proposed Additions

Each proposal is the smallest constraint that closes the finding; none is
authored here, and all remain subject to owner approval.

- **FR** — define the enumeration rule for "reachable store" and require the
  replay to name the rule as well as the population (FND-002); state the
  uniqueness key for an executable path and require rename reconciliation across
  candidate revisions (FND-004); state the uniqueness rule for the `<domain>.<op>`
  dispatch map (FND-012); extend FR-096's realpath, digest-pin, output-floor and
  termination-taxonomy rules to every child process `quoin-core` spawns, ix-flow
  included (FND-007, FND-018); require refusal when a required digest expectation
  is absent rather than merely mismatched (FND-008); require a completeness marker
  for a materialized module root (FND-015); reconcile FR-096's Outputs with its
  non-success contract (FND-020).
- **NFR** — require crash-atomicity, concurrent-invocation semantics and
  orphan-temp handling for every store write (FND-003); extend byte-identity
  verification to newly written records, not only pre-existing ones (FND-009);
  bound the toolchain above as well as below (FND-016); add a precedence rule for
  overlapping allowance entries and a mutual-exclusion rule between allowance and
  retention (FND-013); scope expiry failure to the report rather than to every
  gate lane, or state the grace path (FND-011).
- **StR** — state that a cutover's reversibility lapses at a named point and that
  dependent cutovers are identified before an earlier retained path is deleted
  (FND-005); state that a captured expectation fixture may be produced only from
  the retained implementation and only before its deletion (FND-006); resolve
  whether stored records may embed non-contractual diagnostic text (FND-010);
  state the case-folding and normalization rule that governs type identity
  (FND-014); require symlink-resolved containment for caller-selected roots
  (FND-017); require termination on cyclic or deeply nested digest-citation and
  analysis graphs (FND-019).
- **Traceability** — break the FR-100 / FR-101 / FR-103 dependency cycle and
  align FR-103's upstream declaration with ADR-0003's "parallel and independently"
  statement (FND-001).
