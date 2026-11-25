---
id: SR-967175026302197768900474181941935084766
title: "Code review — Quoin release pins and design matrix errors"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; default-modules.yaml, rust/Cargo.lock, rust/crates/quoin-cli/src/{analysis_evidence.rs,matrix.rs}, rust/crates/quoin-core/src/ops/quire/{mod.rs,wire.rs,tests.rs}, rust/crates/quoin-dossier/{Cargo.toml,src/architecture.rs}, rust/crates/quoin-quire/src/{design_matrix.rs,evidence.rs}; spec/functional/FR-165-matrix-design-column.md, FR-193-dossier-architecture-content.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-165", type: references }
  - { target: "ix://agent-ix/quoin/FR-193", type: references }
---

## Summary

Reviewed Quoin PR #1071 at `2a09297980d6477314c3e0a5b5ba5e9942c1bd25` against `origin/main`, including the version pins, FR-165-AC-21 adapter, its test, and the architecture dossier caller. The new boundary preserves both valid matrix entries and dropped-relation errors, but the architecture dossier consumes only entries and can silently render a partial result.

## Verdict

**CONDITIONAL** — one medium finding remains in the architecture dossier's handling of engine errors.

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies because the change alters information emitted by the matrix/report path. Its `review_policy.mode` is `recommend`; it recommends evidence, integrity and scope-boundary analyses. AP-202 does not apply because this PR does not change the EARS or gap-analysis lens itself. The requested spec-review lens selection is pending the owner.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `ArchitectureGenerator` serializes only `outcome.design_matrix.entries` and ignores the new `errors` collection. When the engine drops a relation but returns partial entries, the dossier can render an incomplete architecture without reporting that dropped relation; if no entry retains a `design` block, it can also emit the inaccurate `design_relations_unavailable` gap. Propagate or report the errors and add a focused dossier test. | rust/crates/quoin-dossier/src/architecture.rs:64 |

## Coverage

- Examined FR-165-AC-21: “Given quire-rs returns at least one valid design-matrix entry and at least one dropped-relation error, `quire.design_matrix` returns the valid entries in `coverage_matrix` and preserves the errors in `design_matrix_errors`.”
- Examined the FR-165 Outputs statement requiring the `quire.design_matrix` payload to retain both entries and errors when they coexist.
- Context only: FR-193 Behavior requiring the dossier architecture document to derive its requirement-to-element mapping from the quire-rs design matrix, and its `design_relations_unavailable` behavior.
- Rust review covered the changed dependency declarations and lock entries, all direct `design_matrix` consumers, the operation wire conversion, the matrix CLI diagnostic path, and the trace-tagged unit test. The computed repository-wide matrix and full repository gates were not run; the release brief prohibits whole-suite or long validation sweeps.
- Focused check evidence from the coder: FR-165 boundary test, default-module manifest test, `cargo check --locked -p quoin-cli -p quoin-dossier`, and `git diff --check` passed via `locked-build.sh`. Reviewer rerun of the FR-165 boundary test passed 1/1 via `locked-build.sh`.
- Reviewer `cargo fmt --all -- --check` exited 1 on formatting differences in unchanged `quoin-cli/src/evidence/audit.rs` and `quoin-cli/tests/tc_1817_suite004_record.rs`; no changed file was listed. Focused `cargo clippy --locked -p quoin-cli -p quoin-core -p quoin-dossier -p quoin-quire --all-targets -- -D warnings` exited 101 with 14 warnings in unchanged `quoin-evidence` files; no changed file was listed, and a baseline clippy comparison was not run. The release brief prohibits full workspace gates and long sweeps; `cargo deny` and the repository-wide matrix were not run.
- No workflow files or gate settings changed. AP-201 applies as a recommended profile; AP-202 does not apply.
