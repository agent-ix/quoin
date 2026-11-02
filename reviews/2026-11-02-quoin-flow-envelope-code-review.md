---
id: SR-186
title: "Code review of Quoin 0.29.3 ix-flow error-envelope release"
type: SpecReview
analysis: code-review
scope: "rust/crates/quoin-cli/src/flow.rs, rust/crates/quoin-measurement-graph/tests/tc_480_fr066_criteria.rs, rust/crates/quoin-quire/Cargo.toml, rust/Cargo.toml, rust/Cargo.lock, .codex-plugin/plugin.json, .claude-plugin/plugin.json, .github/workflows/build-test.yml"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-021"
    type: references
---

## Summary

Reviewed the ix-flow launch boundary, error parsing, success-path refusal behavior, traced tests, engine pin, version metadata, and workspace conventions. The fix reads `ok: false` and returns the provider's error message before asking for `instance_id`; successful envelopes still require the id. AP-201 and AP-202 do not scope this launcher change: it does not alter finding quality or Jev evaluation. The measurement-graph test file contains only rustfmt corrections required by the current main revision's lint gate.

## Verdict

**PASS** — no open code findings. Focused launcher tests, formatting, and Clippy pass; hosted full CI is running on the version-aligned release candidate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Dispositions

FND-001 is the no-findings placeholder; no disposition applies.

## Rust Review

The new `instance_id` helper distinguishes an ix-flow error envelope from a successful response before dereferencing the run id. It preserves the provider's user-facing message, supplies a deterministic fallback for malformed error envelopes, and keeps the existing refusal for success without an id. No public API, unsafe code, persistence shape, or concurrency behavior changes. The tests cover provider error, success with id, and success without id and carry the FR-021 trace tag. `cargo test --locked -p quoin-cli flow::tests::` passes 5 tests. `cargo fmt --all -- --check` passes. The release pins quire-rs 0.50.4 and sets workspace plus Codex/Claude plugin metadata to 0.29.3. The corrected Cargo fetch step downloads dependencies for cross-target offline graph checks.
