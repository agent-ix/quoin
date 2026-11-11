---
id: SR-20261111182227
title: "Code review of quoin PR 805 (FR-130 scenario-evidence intake)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@2747fe6a; rust/crates/quoin-evidence/src/scenario/, rust/crates/quoin-evidence/src/{error,paths,lib}.rs, rust/crates/quoin-core/src/ops/evidence/{records,taxonomy,wire,mod,tests}.rs, rust/crates/quoin-core/src/dispatch{.rs,/tests.rs}, rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs"
review_set: subset
---

# Code review of quoin PR 805

## Summary

Ticket: PLAT-78. Scoped to `git diff origin/main...HEAD` at 2747fe6a. Rust lane
(`rust-review`, with the repo's `.claude/skills/rust-style`) folded in. Ran via
locked-build.sh (debug=0): `cargo test -p quoin-evidence --lib scenario` (10 passed),
`cargo test -p quoin-core --lib` evidence and dispatch filters (16 and 13 passed),
`--test tc_458_evidence_boundary tc_2208` (1 passed), `--test tc_90_module_sizes` (2 passed).
A throwaway probe test (reverted, never pushed) measured the attribution check and the
edge inputs cited below.

Clean: the contract member is read before the typed shape (a v2 document carrying an
unknown member is refused `wrong-contract`, not `InvalidRecord`). The path is built only
from the closed `Integration` enum and a scenario id restricted to `[A-Za-z0-9-]` and
prefixed `<integration>-`, so traversal and separators cannot reach `scenario_path`.
Storage goes through `EvidenceSource::publish` -> `quoin_store::publish` with
`StorePath::ContentAddressed`, the shared hard-link no-replace publisher: an identical
second record answers `created: false`, a different one is `RecordIntegrity` with the first
bytes untouched (TC-2207). Every request type is `deny_unknown_fields`; the payload is a
typed `Serialize` struct. The census edit adds `MAX_SCENARIO_RECORD_BYTES` (4 MiB, same as
the assurance-record ceiling) to the transport-ceiling census, which is required for every
new `MAX_*_BYTES`; the evidence error-code count 9 -> 10 adds `ScenarioRefused` to the
BadRequest arm deliberately. Neither weakens a guard. The 700-line module cap
(`tc_90_module_sizes.rs`) is not edited by this PR; the new modules are 195 and 328 lines.
Nothing in code or spec invents a Quoin revision or pin: the published identity is owner
`quoin` plus contract `quoin.protocol-scenario-intake/v1`; `revision` fields are
producer-authored attributions copied from the document.

## Verdict

**PASS WITH FINDINGS (four medium, two low).** The intake is correct on its main path and
its append-only semantics are sound. The attribution check is quadratic within the
admitted request size, and the scenario id lacks a length bound and a case-canonical form,
so two real edges land as untyped store errors.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Value-owner attribution scans every attributable entry per value owner (O(values x entries)). A 3.7 MB request, under the 4 MiB ceiling, with 30k entries and 30k value owners took 11.1 s in the unoptimised test build. Build a set of (owner, contractVersion, revision) triples once and look each value owner up | rust/crates/quoin-evidence/src/scenario/check.rs:148-174 |
| FND-002 | medium | The scenario id has no length ceiling. `IT-002-` followed by 300 letters passes `path_safe` and becomes a file name over NAME_MAX, so the write fails as a StoreIo refusal rather than a typed QE-E010 refusal. Bound the id (for example 128 bytes) and refuse past it as a named kind | rust/crates/quoin-evidence/src/scenario/check.rs:55-67 |
| FND-003 | medium | The scenario id allows mixed case, so on a case-insensitive filesystem (macOS and Windows release targets) `IT-002-SC-07a` and `IT-002-SC-07A` share one file. The second distinct scenario is refused as RecordIntegrity instead of being filed. Restrict the id to one case or encode it injectively before forming the path | rust/crates/quoin-evidence/src/scenario/check.rs:55-67, rust/crates/quoin-evidence/src/paths.rs:87-89 |
| FND-004 | medium | `Dimension` (20 members) and `Integration` re-declare vocabularies quire-protocol owns (FR-016 `BudgetDimension`, FR-017 integrations) as closed Quoin enums. They match today, but a dimension Protocol adds makes this intake refuse a valid record. The intake decides no sufficiency, so it needs only an exact-label string plus the uniqueness check. Take the label as an opaque non-blank string, or get the owner to confirm that the closed set is part of the intake contract | rust/crates/quoin-evidence/src/scenario/mod.rs:178-206 |
| FND-005 | low | A non-string `contract` member (for example `1`) is refused as `missing-component` with "contract is absent". It is present but names no accepted version, so it should be refused as `wrong-contract` | rust/crates/quoin-evidence/src/scenario/check.rs:23-38 |
| FND-006 | low | A scenario id equal to the bare prefix (`IT-002-`) is admitted and filed as `scenarios/IT-002/IT-002-.json`. Require at least one character after the integration prefix | rust/crates/quoin-evidence/src/scenario/check.rs:96-107 |

## Examined scope

```yaml
clean: false
scope:
- {id: admit, path: rust/crates/quoin-evidence/src/scenario/check.rs, role: examined, excerpt: "match document.get(\"contract\").and_then(Value::as_str) { Some(INTAKE_CONTRACT) => {} ... } let record: ScenarioEvidence = serde_json::from_value(document.clone())"}
- {id: path_safe, path: rust/crates/quoin-evidence/src/scenario/check.rs, role: examined, excerpt: "value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')"}
- {id: check, path: rust/crates/quoin-evidence/src/scenario/check.rs, role: examined, excerpt: "let attributed = attributable.iter().any(|entry| entry.owner == owner.owner && ...)"}
- {id: write_scenario_evidence, path: rust/crates/quoin-evidence/src/scenario/mod.rs, role: examined, excerpt: "let created = source.publish(&path, &bytes)?;"}
- {id: scenario-types, path: rust/crates/quoin-evidence/src/scenario/mod.rs, role: examined, excerpt: "pub enum Integration / ResolutionState / ClaimOutcome / Dimension / ScenarioRefusal"}
- {id: record_scenario, path: rust/crates/quoin-core/src/ops/evidence/records.rs, role: examined, excerpt: "bound(request, op, MAX_SCENARIO_RECORD_BYTES)?; ... write_scenario_evidence(source, &request.document)"}
- {id: map_error, path: rust/crates/quoin-core/src/ops/evidence/taxonomy.rs, role: examined, excerpt: "EvidenceError::ScenarioRefused { kind, .. } => envelope.with_context(\"refusal\", kind.as_str())"}
- {id: census_of_the_evidence_domain, path: rust/crates/quoin-core/src/dispatch/tests.rs, role: examined, excerpt: "(\"ops::evidence::MAX_SCENARIO_RECORD_BYTES\", crate::ops::evidence::MAX_SCENARIO_RECORD_BYTES)"}
- {id: QE-E010, path: rust/crates/quoin-evidence/src/error.rs, role: examined, excerpt: "Self::ScenarioRefused => \"QE-E010\""}
- {id: publish, path: rust/crates/quoin-evidence/src/source.rs, role: context_only, excerpt: "StorePath::ContentAddressed(self.resolve(path)) ... ContentCollision => RecordIntegrity"}
- {id: tc_90_module_sizes, path: rust/crates/quoin-core/tests/tc_90_module_sizes.rs, role: context_only, excerpt: "const HARD_CEILING: usize = 700; (unchanged by this PR)"}
```
