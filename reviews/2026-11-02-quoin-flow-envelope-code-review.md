---
id: SR-186
title: "Code review of Quoin 0.29.3 ix-flow error-envelope release"
type: SpecReview
analysis: code-review
scope: "rust/crates/quoin-cli/src/flow.rs, rust/crates/quoin-measurement-graph/tests/tc_480_fr066_criteria.rs, rust/crates/quoin-quire/Cargo.toml, rust/Cargo.toml, rust/Cargo.lock, .codex-plugin/plugin.json, .claude-plugin/plugin.json, .github/workflows/build-test.yml, rust/crates/quoin-jsonschema/tests/tc_470_formats.rs, tests/fixtures/operational-date-time-probe.json"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-021"
    type: references
---

## Summary

Reviewed the ix-flow launch boundary, error parsing, success-path refusal behavior, traced tests, engine pin, version metadata, and workspace conventions. The fix reads `ok: false` and returns the provider's error message before asking for `instance_id`; successful envelopes still require the id. AP-201 and AP-202 do not scope this launcher change: it does not alter finding quality or Jev evaluation. The measurement-graph test file contains only rustfmt corrections required by the current main revision's lint gate.

## Verdict

**PASS** — no open code findings. The ix-flow regression tests pass, the removed-fixture test failure is repaired, and full hosted CI is green on the version-aligned release candidate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Date-time schema tests referenced an operational evidence file removed from the fixture store, so the workspace test gate failed before reaching later crates. | rust/crates/quoin-jsonschema/tests/tc_470_formats.rs:69 |

## Dispositions

| Finding | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Added a complete minimal operational record fixture and updated the test to use it. The targeted format test and full hosted CI pass. |

## Rust Review

The new `instance_id` helper distinguishes an ix-flow error envelope from a successful response before dereferencing the run id. It preserves the provider's user-facing message, supplies a deterministic fallback for malformed error envelopes, and keeps the existing refusal for success without an id. No public API, unsafe code, persistence shape, or concurrency behavior changes. The tests cover provider error, success with id, and success without id and carry the FR-021 trace tag. `cargo test --locked -p quoin-cli flow::tests::` passes 5 tests. `cargo fmt --all -- --check` passes. The release pins quire-rs 0.50.4 and sets workspace plus Codex/Claude plugin metadata to 0.29.3. The corrected Cargo fetch step downloads dependencies for cross-target offline graph checks. Full hosted CI run 41522089933 passes across Rust, Windows plugin workflows, and supply-chain checks.
