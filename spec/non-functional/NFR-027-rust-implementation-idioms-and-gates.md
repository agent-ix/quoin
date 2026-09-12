---
id: NFR-027
title: "Rust implementation idioms and the gates that enforce them"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-096"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-099"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-100"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-101"
    type: "constrains"
---

# NFR-027: Rust implementation idioms and the gates that enforce them

## Statement

Every crate in the Quoin Rust workspace SHALL forbid `unsafe` code at its root,
SHALL take its lints from one workspace-level declaration, SHALL expose one
`thiserror`-derived error type per crate boundary drawn from a stable code
catalog, and SHALL carry identifiers and resource references as newtypes, and the gates asserting
these SHALL be run rather than assumed.

## Scope

- Applies to: every crate in the `rust/` Cargo workspace, including
  `quoin-core`, `quoin-difftest` and each domain crate, and to their public
  library surfaces.
- Operational context: every candidate revision, in continuous integration and
  in a pre-push run on a maintainer's machine.
- Not applied to: retained TypeScript and its lint configuration; inert Rust
  sample inputs under the corpus, which compile no assertion.

## Rationale

The `/rust-review` skill is the standing review guide for this programme, and
[ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md)
adopts it for this workspace. A review guide is advice; a requirement is a gate.
This requirement promotes the subset of that guidance that a machine can check,
so that a deviation is a failing run rather than a reviewer's recollection.

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

The last clause is the point of §12: a gate that is described and not run is the
same defect this repository already named in
[NFR-020](./NFR-020-declared-toolchain-floors.md) — a green result for a check
that did not execute.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Crate roots declaring `#![forbid(unsafe_code)]` | all | all | Test |
| Crates taking lints from `[workspace.lints]` | all | all | Test |
| Clippy findings under `-D warnings` | 0 | 0 | Test |
| Public library APIs returning `Box<dyn Error>` or an `anyhow` error | 0 | 0 | Test |
| Error types per crate boundary | 1 | 1 | Test |
| Error codes absent from the crate's code catalog | 0 | 0 | Test |
| Public functions taking or returning a bare `String` or `Uuid` identifier | 0 | 0 | Test |
| New tests without a tracking tag resolving to a criterion | 0 | 0 | Test |

## Verification

The gate runs, and records the output of, each of `cargo fmt --check`;
`cargo clippy --workspace --all-targets --all-features -- -D warnings`;
`cargo test --workspace`; and `cargo deny check` where `rust/deny.toml` exists. A
source check asserts `#![forbid(unsafe_code)]` on every crate root, `[lints]
workspace = true` in every crate manifest with the policy defined once at the
workspace root, exactly one `thiserror`-derived error type on each crate
boundary, every error code present in that crate's checked-in code catalog, no
`Box<dyn Error>` or `anyhow` type in a public signature, and no bare `String` or
`Uuid` identifier parameter or return in a public signature. A test check asserts
every test function carries a tracking tag whose cited criterion ids resolve to
an artefact in `spec/`. Planted violations of each rule must fail the
corresponding check; a check whose scanned population is empty is reported as
inconclusive rather than as clean. A change that removes a gate lane, narrows
one, or adds `continue-on-error` fails this requirement independently of the code
it accompanies.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-027-AC-1 | Every crate root declares `#![forbid(unsafe_code)]`, and a planted `unsafe` block fails the build. | Test (TC-1674) |
| NFR-027-AC-2 | The lint policy is declared once in `[workspace.lints]` at `rust/Cargo.toml`, every crate manifest opts in with `[lints] workspace = true`, and a per-crate override fails the gate. | Test (TC-1675) |
| NFR-027-AC-3 | `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` both exit zero, and their output is recorded with the run. | Test (TC-1676) |
| NFR-027-AC-4 | Each crate boundary exposes exactly one `thiserror`-derived error type whose every code appears in that crate's checked-in code catalog, and a code absent from the catalog fails the gate. | Test (TC-1677) |
| NFR-027-AC-5 | No public library signature returns `Box<dyn Error>` or an `anyhow` error, and a planted one fails the gate. | Test (TC-1678) |
| NFR-027-AC-6 | No public library signature takes or returns a bare `String` or `Uuid` as an identifier or resource reference, and a planted one fails the gate. | Test (TC-1679) |
| NFR-027-AC-7 | Every Rust test function carries a tracking tag whose cited criterion ids resolve to an artefact under `spec/`, and an untagged or unresolvable-tag test fails the gate. | Test (TC-1680) |
| NFR-027-AC-8 | A continuous-integration change that removes a gate lane, narrows one, or adds `continue-on-error` fails this requirement's check. | Test (TC-1681) |
| NFR-027-AC-9 | Each of the source and test checks reports the population it scanned, and a check that scanned nothing is reported as inconclusive rather than as passing. | Test (TC-1682) |

## Dependencies

- **Upstream**: [NFR-026](./NFR-026-rust-toolchain-floor.md), which pins the toolchain these gates run on; [ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md), which adopts `/rust-review` as the workspace review guide.
- **Downstream**: [FR-101](../functional/FR-101-retire-replaced-executable-paths.md), whose restated Rust criteria must carry the tracking tags this requires.
