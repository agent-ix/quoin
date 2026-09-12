---
id: SR-151
title: "Base review of the Rust burn-down spec set"
type: SpecReview
analysis: base
scope: "ADR-0003, StR-009, US-024, FR-096..FR-103, NFR-024..NFR-027, IT-003"
review_set: all
---

# Base review of the Rust burn-down spec set

## Summary

EPIC #373 adds one architecture decision, one stakeholder requirement, one user
story, eight functional requirements, four non-functional requirements and one
integration test for the staged Rust port of Quoin's first-party engine and
qualification logic behind a versioned `quoin-core` subprocess boundary. All
fifteen artefacts pass `quire validate` with zero structural errors. After the
seven analyses were resolved the batch allocates TC-1601 through TC-1709
contiguously, and no test exists for any of them yet: the Test Matrix has not
been rebuilt, which is the expected state between authoring and the matrix step,
and is recorded here rather than reported as coverage.

## Checklist Results

| Area | Result | Evidence |
|---|---|---|
| ID format and uniqueness | Pass | StR-009, US-024, FR-096..FR-103, NFR-024..NFR-027, IT-003, ADR-0003 continue the quoin sequences; `git ls-tree` over every remote branch confirms no other branch claims StR-009, US-024, FR-096..FR-100, FR-102, FR-103, NFR-024..NFR-027, IT-003 or ADR-0003 |
| ID collision | Fail | `origin/spec/287-catalog-locks` at 1b5caa9 already carries `FR-101-lock-semantic-module-and-package-revisions.md` and `US-023`; this set takes FR-101 because the epic's spec-artifact table and AC-2 name it, and the collision needs an owner ruling (FND-001) |
| Sequential allocation | Pass | FR-096..FR-103 contiguous; NFR-024..NFR-027 contiguous; TC-1601..TC-1690 contiguous with no gap and no reuse |
| Stakeholder requirement quality | Pass | StR-009 states the need normatively with a named agent, carries rationale, five validation criteria with methods, stakeholders, assumptions, constraints, dependencies, priority and traceability |
| User story quality | Pass | US-024 has the As a / I want / So that shape, five illustrative examples, options, contextual constraints, dependencies, priority and traceability; it prescribes no mechanism |
| Functional requirement quality | Pass | Each of FR-096..FR-103 carries Description, Inputs, Outputs, Behavior, Error Conditions, a Constraints table with validation methods, an Acceptance Criteria table and Dependencies |
| Non-functional requirement quality | Pass | NFR-024..NFR-027 each carry Statement, Scope, Rationale, a Measurement and Evaluation table with target and threshold, Verification naming the command, an Acceptance Criteria table and Dependencies |
| Integration test quality | Pass | IT-003 carries Objective, Target Integration, Preconditions, Inputs, a five-step Test Procedure with IT-003-SC-01..05, Expected Results, Metadata, Dependencies, Notes and Traceability |
| Criteria verifiability | Partial | Fourteen criteria name a command outright (`make test`, `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo tree`, `cargo fetch`, `pnpm install --frozen-lockfile`, `quoin-core lint.removal`, `quoin-core lint.language`); the remainder name a planted-violation outcome or a counted population but not the command that produces it. Counted in SR-155 FND-002 and deferred to the matrix step, which binds each to a test |
| Error conditions documented | Pass | Each FR carries an Error Conditions section; refusal classification is a criterion in FR-096, FR-098, FR-099 and FR-100 |
| Coverage (Rule 1) | Not yet assessable | Every criterion carries a TC reference or a named non-test method; no TC is backed by a test, because the matrix step has not run (FND-002) |
| Option permutation (Rule 2) | Not yet assessable | Deferred to the matrix step |
| Constraint boundary (Rule 3) | Pass | 29 constraints across the eight FRs, each with a validation method; boundary values are stated where they exist (67,108,864-byte payload under a strictly greater ceiling, exit statuses 0 through 4, Rust channel 1.98.1, `thin-host` line and branch ceilings) |
| Error path (Rule 4) | Pass | Each FR's Error Conditions enumerate the refusals, and the negative cases are criteria rather than prose: planted duplicate type, planted hand edit, planted second canonicalization, planted retention with no successor, planted `unsafe`, planted empty population |
| Edge case (Rule 5) | Pass | The empty-population case, the self-written-fixture case, the expired-retention case and the submodule-ownership case are each a criterion |
| Cross-referencing | Pass | Every FR relates to StR-009; NFRs constrain the FRs they bound; IT-003 verifies FR-096 and FR-097; body references are relative-path links per ADR-0007 and ix:// only for cross-repository targets |
| Terminology | Pass | No term is coined; retained-with-successor, allowance, candidate revision, disposition, containment and burn-down are used as the implementation language policy and EA ADR-002 define them |
| Quire validation | Pass | Zero structural errors across the batch; grammar warnings are advisory and enumerated in the ears-conformance review |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-101 and the id US-023 are already claimed on the unmerged branch `origin/spec/287-catalog-locks` (1b5caa9, 2026-09-04, no PR). This set takes FR-101 because EPIC #373's spec-artifact table and AC-2 name it explicitly, and takes US-024 to leave US-023 alone. Whichever artefact keeps FR-101 needs an owner ruling before either branch merges. | FR-101, `origin/spec/287-catalog-locks` | missing-requirement |
| FND-002 | medium | No acceptance criterion in the batch is backed by a test. TC-1601..TC-1690 are allocated but the Test Matrix has not been rebuilt, so coverage rules 1 and 2 cannot be assessed. This is the expected state between authoring and the matrix step and is recorded so it is not mistaken for coverage. | spec/matrix.md, all batch artefacts | correct-requirement-no-evidence |
| FND-003 | medium | NFR-026 sets the workspace floor at Rust 1.98.1 on the ground that `engineering-assurance` and `quire-corpus` declare it, but `filament-core-data` and `quire-rs` both pin 1.94.1 in their `rust-toolchain.toml`. EPIC #373 states the floor is "consistent with quire-corpus / filament-core-data", which is measurably false for fcd. NFR-026-AC-4 requires the disagreement to be reported rather than absorbed; raising another repository's floor is an owner decision. | NFR-026, `filament-core-data/rust-toolchain.toml`, `quire-rs/rust-toolchain.toml` | wrong-requirement |
| FND-004 | medium | NFR-024 requires every retained path to name a valid successor reference, defined as an open sub-issue of the burn-down epic naming its delivery stage and the path it retires. No such issue exists: EPIC #373 carries stages 0 through 9 in prose only. Read strictly, every retained path fails allowance 5 on the day the enforcement lands, so the metric's opening value is dominated by this. Creating the stage tickets is a prerequisite to the first enforcement run. | NFR-024, EPIC #373 | missing-requirement |
| FND-005 | medium | Three criteria are verified by Inspection alone — FR-097-AC-7 and FR-102-AC-7 on registry declarations, FR-103-AC-6 on relocation into `quire-corpus`. Registry declarations are mechanically checkable from the manifests and should be promoted to Test at the matrix step; the relocation criterion legitimately reaches another repository and may stay Inspection. | FR-097, FR-102, FR-103 | correct-requirement-no-evidence |
| FND-006 | low | FR-103 is scoped to the Quoin side of corpus tooling because `corpus/` is a git submodule pointing at `agent-ix/qa-corpus` at 7b81343. Whether this programme has any scope over that repository is unresolved and is recorded as an open question in ADR-0003 rather than assumed either way. | FR-103, ADR-0003 | missing-requirement |
| FND-007 | low | ADR-0003 is `status: proposed`. FR-096 and FR-101 both declare an accepted ADR-0003 upstream, so no delivery stage may start until the owner accepts it on issue #373. | ADR-0003, FR-096, FR-101 | correct-requirement-no-evidence |

## Resolution log

The seven analyses (SR-152..SR-158) returned 129 findings over this batch. The
table below records what was resolved in the artefacts and what was not. A row
marked *owner* is not a defect the author may close.

| Finding | Disposition |
|---|---|
| Prerequisite cycle FR-100 → FR-101 → FR-103 → FR-100 (SR-152 FND-001, SR-153 FND-001, SR-154 FND-001) | Resolved. FR-103 no longer declares FR-101 upstream; it depends on FR-096 and FR-084 and runs in parallel, matching ADR-0003. |
| Cycle-break stated twice, ordered backwards (SR-154 FND-002) | Resolved. The break belongs to FR-100 only; the duplicate bullet and the cycle clause of FR-101-AC-7 were removed. |
| NFR-024 declared downstream of the requirements that read it (SR-154 FND-003) | Resolved. NFR-024 is upstream of FR-100, FR-101 and FR-103; those three now name it. |
| Allowance manifest path contradicted itself (SR-153 FND-004, SR-155 FND-005, SR-156 FND-183) | Resolved. `.language-allowances.yaml` at the repository root, everywhere. |
| Manifest categories numbered 1–4 while allowances 5 and 6 exist (SR-153 FND-002, SR-155 FND-005) | Resolved. NFR-024 now names the manifest's own categories — `ui`, `generated`, `inert`, `thin-host`, `owner-disposition` — and states that staged-port retention lives in the burn-down matrix, not the manifest, because it is counted. |
| The successor rule refuses the matrix that exists (SR-155 FND-004) | Resolved. A `quoin#373 Stage N` reference is reported as **provisional**, distinct from valid and invalid, satisfying nothing (NFR-024-AC-10). |
| Delivery-stage tickets 0–9 do not exist (SR-153 FND-003, SR-154 FND-013, SR-156 FND-183) | Partly resolved. ADR-0003 records that they are created before the first enforcement run; creating them is programme work, not a spec edit. |
| Overlapping globs, dual classification (SR-152 FND-013) | Resolved. NFR-024-AC-11: more specific glob wins, equal specificity fails, a path may not hold both a manifest entry and a retention row. |
| Population excludes extension-less executable paths (SR-155 FND-008) | Resolved. FR-101-AC-10 and NFR-024-AC-12 classify Makefile recipes and workflow `run:` blocks by role. |
| Expiry is a calendar time bomb (SR-152 FND-011) | Resolved. NFR-024-AC-13: an expired retention fails the report, not the build lane. |
| Enforcement run has two claimants (SR-157 FND-001) | Resolved. LR08 owns the run; Quoin owns retention of its result. Stated in ADR-0003, FR-100 and NFR-024. |
| Canonical serialization allocated both to `engineering-assurance` and to `quoin-store` (SR-157 FND-007) | Resolved. Removed from the EA-consumed list; `quoin-store` owns it, as the evidence-store exception. |
| sha256 record identity is the identity that matters and was unnamed (SR-156 FND-175) | Resolved. FR-098 and FR-100 name sha256 record identity alongside JCS and blake3; FR-098-AC-10 asserts identifier and filename byte-identity. |
| Strict JSON parser refusal boundary not ported (SR-156 FND-176) | Resolved. FR-098 ports and compares it; FR-098-AC-9. |
| ajv asserts no formats; strict-mode diagnostics have no Rust counterpart (SR-156 FND-174, FND-177) | Resolved. FR-098 states format assertion off and excludes schema-authoring strict-mode diagnostics; FR-098-AC-8. |
| "Every reachable store" undefined (SR-152 FND-002, SR-156 FND-182) | Resolved. FR-098, FR-100 and NFR-025 define it as the evidence store root plus each root in the checked-in store inventory; an absent inventory refuses rather than narrows. |
| Forward divergence in newly written records unguarded (SR-152 FND-009) | Resolved. NFR-025 scope now covers newly written records; NFR-025-AC-6. |
| No crash, concurrency or torn-record rule (SR-152 FND-003) | Resolved. FR-100 requires temp-file plus atomic rename, orphan cleanup and torn-record refusal; FR-100-AC-9. |
| Fixture provenance unconstrained; recapture from Rust possible (SR-155 FND-009) | Resolved. FR-101 requires producing implementation, revision and request digest per fixture and refuses a Rust-produced fixture; FR-101-AC-11. |
| Self-written-fixture gate named no command and was too narrow (SR-155 FND-010) | Resolved. FR-101-AC-6 names `quoin-core lint.removal` and covers setup helpers and generation steps. |
| ix-flow is a second unhardened trust boundary (SR-152 FND-007) | Resolved. FR-099 applies the FR-096 hardening to the ix-flow child; FR-099-AC-10. |
| Unset `QUOIN_EXPECTED_CORE_SHA256` was a vacuous pass (SR-152 FND-008) | Resolved. FR-096 refuses when unset; FR-096-AC-5. |
| Root escape via symlink (SR-152 FND-017) | Resolved. FR-096 real-path-resolves the repository and configuration roots. |
| 67,108,864-byte payload sits exactly on the retained cap (SR-156 FND-178) | Resolved. FR-096 requires a ceiling strictly greater than the payload. |
| Partially materialized module root; case-folding collision (SR-152 FND-015, FND-014) | Resolved. FR-099-AC-11 and FR-099-AC-12. |
| ix-cli-core symbol count wrong and asserted over the wrong manifest (SR-156 FND-179) | Resolved. FR-099 names the twelve measured symbols and asserts over `package.json` against the inventory rather than a hard-coded count. |
| Path identity not rename-stable (SR-152 FND-004) | Resolved. FR-101-AC-12. |
| FR-097 mixed Phase A and Phase B work (SR-154 FND-004) | Resolved. Generation and provenance are Phase A; only duplicate retirement is Phase B. |
| IT-003 gated all of FR-101 (SR-154 FND-005) | Resolved. It now gates the duplicate-retirement clauses of FR-097 only. |
| Baseline figures revision-bound (SR-153 FND-017, SR-156 FND-185) | Resolved. FR-101-AC-9 asserts the figures *of* `e718d45`, not of the revision under test. |
| Test-oracle population stated as 109 files (SR-156 FND-186) | Resolved. 105 `*.test.ts`, seven `fast-check` suites, in ADR-0003 and US-024. |
| FR-098 cited a corpus pin `src/quire/exec.ts` does not carry (SR-155 FND-017) | Resolved. Replaced by a checked-in differential manifest pinning repository and revision. |
| Differential harness would outlive FR-101-CON-4 (SR-153 FND-014) | Resolved. FR-098 retires the live oracle at cutover; FR-098-AC-11. |
| NFR-026 gates unconstructible or over-broad (SR-155 FND-012, SR-156 FND-181, SR-153 FND-015) | Resolved. AC-2 uses a `RUSTUP_TOOLCHAIN` override, AC-3 scopes to authored files with a reported population, AC-4 reads a checked-in record and no network. |
| NFR-026 bounded only from below (SR-152 FND-016) | Resolved. The gate refuses a toolchain other than the declared channel without a recorded decision. |
| `corpus/` submodule guarantees and dispositions (SR-155 FND-007, SR-157 FND-008) | Resolved. FR-103 and FR-101 forbid only a *port or delete* disposition inside the submodule; an inert entry is permitted. NFR-025 excludes the submodule's contents from its byte guarantee. |
| `skills/` classification (SR-155 FND-006, SR-156 FND-173) | Partly resolved. FR-099, FR-101 and ADR-0003 now distinguish the 139-line first-party invariant shim from the 22,575 lines of vendored `ix-spec-workflows` bundle, which claims the `generated` exception only with recorded provenance. Whether the bundle belongs in the baseline at all is **owner**. |
| Missing frontmatter edges; US-024 reached only by FR-096 (SR-154 FND-007, SR-153 FND-018) | Resolved. FR-097..FR-103 declare `implements US-024`; the frontmatter now carries the prose edges. |
| Inspection-only criteria for mechanically checkable facts (SR-153 FND-016, SR-155 FND-016) | Partly resolved. FR-097-AC-7, FR-102-AC-7 promoted to Test. The seven Inspection constraint rows remain Inspection. |
| `engineering-assurance` is "a reference, not a dependency" yet mandatory (SR-154 FND-006, SR-157 FND-002) | **Owner.** The direction of that edge is a cross-repository decision. |
| `@agent-ix/quoin` publishes to public npmjs today (SR-156 FND-180) | **Owner.** FR-102 now routes the registry through a dated owner disposition instead of asserting a change. |
| `filament-core-data` and `quire-rs` pin 1.94.1 (SR-153 FND-013, SR-156 FND-181) | **Owner.** NFR-026 states Quoin's floor and requires the disagreement reported. |
| FR-101 / US-023 id collision on `origin/spec/287-catalog-locks` (FND-001 above) | **Owner.** |
| `agent-ix/qa-corpus` scope (FND-006 above, SR-155 FND-007) | **Owner.** |
| Delivery-stage tickets, allowance manifest and matrix must be created (SR-153 FND-003, SR-155 FND-004/005) | **Programme work**, tasked at the plan step, not resolvable in the spec. |
| EARS non-singular statements, 43 measured warnings plus the wrap-hidden cases (SR-158) | Partly resolved. The three `before` triggers are canonical now and several conjunctions were split. The remaining non-singular statements are advisory warnings that do not block validation and are deferred to a dedicated pass. |
| Coverage rules 1 and 2, evidence declarations, property domains (FND-002 above, SR-155 FND-001/002/003/011/013/014/015) | **Deferred to the matrix step**, which binds TC ids to tests and is where an unbacked criterion becomes visible. |

`quire validate` over the batch after resolution: zero structural errors.
