---
id: SR-20261117175248
title: "Code review — quoin M3 dossier generators"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-dossier/src/{product.rs,gap.rs,lib.rs}, rust/crates/quoin-dossier/schemas/*-source-v1.schema.json, rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs, rust/crates/quoin-dossier/Cargo.toml, rust/Cargo.lock; spec/functional/FR-191, FR-229..FR-233"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-191", type: references }
  - { target: "ix://agent-ix/quoin/FR-229", type: references }
  - { target: "ix://agent-ix/quoin/FR-230", type: references }
  - { target: "ix://agent-ix/quoin/FR-231", type: references }
  - { target: "ix://agent-ix/quoin/FR-232", type: references }
  - { target: "ix://agent-ix/quoin/FR-233", type: references }
---

## Summary

Reviewed frozen PR #1068 at `25a9c9d1` against FR-191 and FR-229–233, including its Rust implementation and focused test evidence. The review found an invalid architecture relationship selection path, missing linked software source identity in DOS-DEV, and a misbound trace tag; the source reader also duplicates the closed record contract already held by its three JSON Schemas.

## Verdict

**FAIL** — the SDS can omit valid connections and render unresolved or wrong-kind connections as valid, and the input contract is declared twice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | SDS compares connection endpoints to selected part ids, although architecture connection ends identify ports; valid connections are omitted, while an endpoint equal to a part id can let a missing or wrong-kind opposite end render without an unresolved-reference gap. | rust/crates/quoin-dossier/src/product.rs:854 |
| FND-002 | medium | DOS-DEV resolves linked software records but its Sources section lists only device_description records, omitting the software source artifact ids and types that it read. | rust/crates/quoin-dossier/src/product.rs:611 |
| FND-003 | high | The device association test is tagged FR-232-AC-1 but exercises only a missing software reference, which is FR-232-AC-3; neither criterion has a correctly tagged behavioral test. | rust/crates/quoin-dossier/src/product.rs:1164 |
| FND-004 | high | The Rust Software, Decision, Device, and SourceRef structs repeat the source-record field, requiredness, and type contract in the three JSON Schemas, creating two maintained definitions of the same closed input contract. | rust/crates/quoin-dossier/src/product.rs:28; rust/crates/quoin-dossier/schemas/software-description-source-v1.schema.json:4 |

## Coverage

- Examined the FR-191 command behavior for source failure, M3 registration/order, producer identity and source identity, and all acceptance criteria in FR-229–FR-233.
- Rust review covered input validation, path and symlink boundaries, schema output, record and domain identity uniqueness, source-reference resolution, determinism, ordering, error propagation, partial-write behavior, public API surface, tests, lint configuration, and changed crate dependencies.
- Focused evidence in `/tmp/plat1838-logs/`: the final dossier no-deps Clippy log and format log pass; final product unit, registry, CLI selector, malformed-source, and duplicate-domain-identity logs pass. Whole-dependency Clippy failed on unchanged `quoin-evidence` lints. A reviewer rerun of workspace `cargo fmt --check` found unrelated formatting differences in `quoin-cli/src/evidence/audit.rs` and `tc_1817_suite004_record.rs`; changed-file `rustfmt --check` passed. A focused all-features Clippy rerun could not build because `quoin-semantic/build.rs` requires absent root `node_modules/@agent-ix/semantic-core/generated/json-schema`; no aggregate suite or mutation was run.
- `cargo deny` was not run because this review brief limits local checks to focused checks.
- No applicable AssuranceProfile was present. No CI workflow or gate-narrowing change is in the PR diff.
- The independent gap-analysis artifact records the computed matrix and test-trace coverage.

## Dispositions

Not applicable in the review pass; findings remain open for the coder fix round.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Current code still selects parts by `owner` and matches endpoints as synthetic `owner.direction` strings instead of the authored part and port artifact ids. Although the new endpoint cardinality check rejects malformed endpoints, valid artifact-id references remain unresolvable. The fix-round test uses the same synthetic identities and bypasses `render_sds`, so it does not exercise this production path. |
| FND-002 | fixed | The DOS-DEV renderer now adds each resolved linked software artifact id to `source_rows` and emits it in Sources. `source_rows.insert(format!("software_description/{source_id}"));` |
| FND-003 | fixed | The missing-software-reference test is now tagged FR-232-AC-3, and the new association contract test separately covers FR-232-AC-1. `#[trace("FR-232-AC-1")]` |
| FND-004 | fixed | The reader now stores validated records as `serde_json::Value`; the three JSON Schemas remain the single closed record contract. `use serde_json::Value;` |
