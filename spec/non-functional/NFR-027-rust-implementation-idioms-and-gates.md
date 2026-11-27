---
id: NFR-027
title: "Rust implementation idioms and their verification"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-096"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-099"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-100"
    type: "constrains"
---

# NFR-027: Rust implementation idioms and their verification

## Statement

Every crate in the Quoin Rust workspace SHALL forbid `unsafe` code at its root,
SHALL take its lints from one workspace-level declaration, SHALL expose one
`thiserror`-derived error type per crate boundary drawn from a stable code
catalog, and SHALL carry identifiers and resource references as newtypes. Each
candidate revision SHALL run the workspace build, lint, test, and dependency
gates, and its release review SHALL inspect the public API and test-trace
population named in this requirement.

## Scope

- Applies to: every crate in the `rust/` Cargo workspace, including
  `quoin-core` and each domain crate, and to their public
  library surfaces.
- Operational context: every candidate revision, in continuous integration and
  in a pre-push run on a maintainer's machine.
- Not applied to: retained TypeScript and its lint configuration; inert Rust
  sample inputs under the corpus, which compile no assertion.

## Rationale

The `/rust-review` skill is the standing review guide for this workspace.
This requirement records executable build, lint, test, and dependency gates,
plus the specific source and trace-tag checks that the release review records.

Each item is here because its absence has a named failure. An error variant whose
payload is a human-readable string moves the discriminant into prose, and the
callers that must distinguish refusals end up comparing message text — which
[FR-096](../functional/FR-096-versioned-rust-engine-boundary.md) forbids at the
boundary and which a stable code catalog prevents inside it. `Box<dyn Error>` or
`anyhow` on a public library API erases that catalog at exactly the seam a
consumer reads. Bare `String` ids let two identifiers be swapped at a call site
with no compiler objection, in a repository whose identifiers are digests.
Per-crate lint configuration lets one crate quietly weaken what the workspace
enforces.

Per the skill's own §0, a repository idiom document outranks the skill. Quoin has
none for Rust today; the workspace stage creates
`.claude/skills/rust-style/SKILL.md`, and until it exists `/rust-review` is the
authority. This requirement is the enforceable floor under both.

The final gate-run record distinguishes an executed check from one that was
described but never run.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Crate roots declaring `#![forbid(unsafe_code)]` | all | all | Test |
| Crates taking lints from `[workspace.lints]` | all | all | Test |
| Clippy findings under `-D warnings` | 0 | 0 | Demonstration |
| Public library APIs returning `Box<dyn Error>` or an `anyhow` error | 0 | 0 | Inspection |
| Error types per crate boundary | 1 | 1 | Inspection |
| Error codes absent from the crate's code catalog | 0 | 0 | Inspection |
| Public functions taking or returning a bare `String` or `Uuid` identifier | 0 | 0 | Inspection |
| New tests without a tracking tag resolving to a criterion | 0 | 0 | Inspection |

## Verification

The gate runs, and records the output of, each of `cargo fmt --check`;
`cargo clippy --workspace --all-targets --all-features -- -D warnings`;
`cargo test --workspace`; and `cargo deny check` where `rust/deny.toml` exists.
The release review records an inspection of crate roots and manifests, error
types and code catalogs, public signatures, and test trace tags. The recorded
review names the inspected population; an empty population is inconclusive.
A change that removes a gate lane, narrows one, or adds `continue-on-error`
is recorded as a review finding independently of the code it accompanies.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-027-AC-1 | Every crate root declares `#![forbid(unsafe_code)]`, and a planted `unsafe` block fails the build. | Test |
| NFR-027-AC-2 | The lint policy is declared once in `[workspace.lints]` at `rust/Cargo.toml`, every crate manifest opts in with `[lints] workspace = true`, and a per-crate override fails the gate. | Test |
| NFR-027-AC-3 | `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` both exit zero, and their output is recorded with the run. | Demonstration |
| NFR-027-AC-4 | Inspection of each crate boundary confirms exactly one `thiserror`-derived error type and verifies every code against that crate's checked-in code catalog. | Inspection |
| NFR-027-AC-5 | Inspection of public library signatures finds no `Box<dyn Error>` or `anyhow` error return. | Inspection |
| NFR-027-AC-6 | Inspection of public library signatures finds no bare `String` or `Uuid` used as an identifier or resource reference. | Inspection |
| NFR-027-AC-7 | Inspection of the Rust test population confirms every test function carries a trace tag resolving to an artefact under `spec/`. | Inspection |
| NFR-027-AC-8 | A continuous-integration change that removes a gate lane, narrows one, or adds `continue-on-error` fails this requirement's check. | Test |
| NFR-027-AC-9 | Each of the source and test checks reports the population it scanned, and a check that scanned nothing is reported as inconclusive rather than as passing. | Test |

## Dependencies

- **Upstream**: the `/rust-review` workspace review guide.
- **Downstream**: none.
