---
id: NFR-026
title: "Declared Rust toolchain floor for the Quoin workspace"
type: NFR
quality_attribute: compatibility
relationships:
  - target: "ix://agent-ix/quoin/FR-096"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-099"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-100"
    type: "constrains"
---

# NFR-026: Declared Rust toolchain floor for the Quoin workspace

## Statement

The Quoin Rust workspace SHALL declare its toolchain channel in one
`rust-toolchain.toml` at `rust/rust-toolchain.toml`, SHALL set that channel to
1.98.1, and SHALL fail every Rust gate naming the required and observed versions
when the running toolchain is older than the declared channel.

## Scope

- Applies to: every crate in the `rust/` Cargo workspace, every Rust gate this
  repository runs, and the continuous-integration lanes that run them.
- Operational context: a maintainer's machine with an arbitrary toolchain
  installed, and a clean runner with none.
- Not applied to: the retained TypeScript toolchain floors, which
  [NFR-020](./NFR-020-declared-toolchain-floors.md) already covers; and
  toolchains declared by other repositories, which those repositories own.

## Rationale

[NFR-020](./NFR-020-declared-toolchain-floors.md) established this repository's
rule for external commands: a floor nobody states is a floor nobody can meet on
purpose, and a check that passes with its tool absent is the defect this
programme exists to end. A Rust workspace has the same failure shape with a
sharper edge, because `rustfmt` and `clippy` output is version-dependent: a
formatter upgrade moves the fixed point and a lint upgrade moves the finding set,
so a gate run on an undeclared toolchain is a claim about whichever toolchain
happened to be on the path.

1.98.1 is chosen because it is the floor `engineering-assurance` — the reference
port whose pattern this set mirrors — and `quire-corpus` already declare, and
because it is the higher of the two floors in use across the ecosystem, so
adopting it requires no other repository to move.

The ecosystem does not currently agree. `quire-rs` and `filament-core-data` both
pin 1.94.1. `quoin-quire` takes a Cargo edge to `quire-rs` and `filament-core-data`
publishes the types this workspace consumes, so the disagreement is a real
compatibility surface and not a documentation inconsistency. This requirement
states Quoin's floor and requires the disagreement to be reported rather than
absorbed; raising another repository's floor is that repository's decision.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Files declaring the workspace toolchain channel | 1 | 1 | Test |
| Declared channel | 1.98.1 | 1.98.1 | Inspection |
| Rust gates that pass on a toolchain older than the floor | 0 | 0 | Test |
| Cross-repository toolchain disagreements reported rather than absorbed | all | all | Test |
| Gate runs on a toolchain above the declared channel without a recorded decision | 0 | 0 | Test |

## Verification

The gate reads the channel from `rust/rust-toolchain.toml`, compares it to the
version reported by the running `rustc`, and fails naming both when the running
version is lower. A second check asserts the channel appears in exactly one file,
so the floor cannot drift between a manifest and a workflow, over a scanned
population the gate reports. A third check compares the declared channel against
a checked-in record of the channels declared by the repositories this workspace
takes a Cargo edge to, reads no network, and reports each disagreement with both
versions named. Because `rustfmt` and `clippy` output is version-dependent above
the floor as well as below it, the gate also refuses a run on a toolchain other
than the declared channel unless a recorded decision permits it. Running `cargo fmt --check` and
`cargo clippy --workspace --all-targets --all-features -- -D warnings` under a
toolchain below the floor must fail rather than warn.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-026-AC-1 | The workspace toolchain channel is declared in exactly one file, `rust/rust-toolchain.toml`, and its value is 1.98.1. | Test (TC-1670) |
| NFR-026-AC-2 | A toolchain gate run with `RUSTUP_TOOLCHAIN` overriding the declared channel to an older version fails before `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` run, naming the required and the observed version. | Test (TC-1671) |
| NFR-026-AC-3 | A second declaration of the channel in a file this repository authors — a workflow, a Makefile or a documentation table — fails the gate; vendored trees and inert sample inputs are excluded from the scanned population, which the gate reports. | Test (TC-1672) |
| NFR-026-AC-4 | The checked-in record of each upstream repository's declared channel is compared against this workspace's channel, and a disagreement is reported naming both repositories and both versions rather than silently accepted; the gate reads the checked-in record and performs no network access. | Test (TC-1673) |

## Dependencies

- **Upstream**: [NFR-020](./NFR-020-declared-toolchain-floors.md), which establishes this repository's toolchain-floor pattern; `ix://agent-ix/engineering-assurance/NFR-005`, whose floor this adopts.
- **Downstream**: [NFR-027](./NFR-027-rust-implementation-idioms-and-gates.md), whose gates run on this toolchain.
