---
id: SR-1854002
title: "Integrity review of FR-029 dropped-declaration refusal (PLAT-1854)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-029-consume-the-quire-json-contract.md"
review_set: subset
---

# Integrity review of FR-029 dropped-declaration refusal (PLAT-1854)

## Summary

Ticket: PLAT-1854. PR agent-ix/quoin#1036, head 3a65d0af. This review checks the new FR-029 text for consistency and completeness against what it depends on: quire-rs `origin/main` at b6beef4 (#715: `DroppedDeclarations`, `DeclarationKind`, `Registry::require_complete_model`) and every quoin `origin/main` call site that computes over the merged traceability model. The measurements are below.

- **QQ-1022 is free.** `ErrorCode::as_str` allocates QQ-1020 (TraceabilityModelUndeclared) and QQ-1021 (ExtractionUnavailable) in the traceability-model block. Nothing in the workspace uses QQ-1022, so it is the next code in that block.
- **Refused is the right class.** `taxonomy.rs` has three classes: Refused (2), BadRequest (3) and Io/Internal (4). There is no Invalid class. QQ-1020 is Refused, and a dropped declaration is a repository-side condition, so Refused is correct. The ticket's "Invalid-class" wording has no counterpart in the taxonomy.
- **The engine.** `CoverageError`, `RelationReportError` and `SearchError` each have a `DeclarationsDropped` variant. `compute_coverage` checks `require_complete_model()` before `ModelUndeclared`. `Registry::traceability()` returns `None` when the model `is_empty()`, and `is_empty()` does not count `dropped`. A whole unreadable section is recorded as kind `traceability_section` with item `traceability` (src/loader/manifest.rs:680).
- **Locked engine.** quoin locks quire-rs v0.62.2, and no quire-rs tag contains #715 yet. Implementing this needs a quire-rs release and bump. That is not a spec defect.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-029-AC-15 and Behavior bullet 1 say a whole unreadable `traceability:` section appears with the *item* `traceability_section`. In the engine, `traceability_section` is the *kind* (`DeclarationKind::TraceabilitySection.as_str()`) and the item is `traceability`. Because the error carries the engine's records "in the engine's order", an implementation that passes them through fails AC-15, and one that satisfies AC-15 must rewrite engine data. | spec/functional/FR-029-consume-the-quire-json-contract.md:53,95; quire-rs src/traceability/prune.rs:76; quire-rs src/loader/manifest.rs:680 | wrong-requirement |
| FND-002 | medium | The command list in Behavior bullet 3 and AC-17 is presented as closed ("These commands are:") but misses commands that also compute over the merged model through the `quire.coverage`, `quire.design_matrix` or `quire.design_join` ops: `quoin advise` (advise.rs:35), `quoin anomaly check` (anomaly.rs:277), `quoin evidence record` (evidence.rs:127), `quoin evidence audit` and `quoin class obligations` (evidence/audit.rs:130, class_obligations.rs:100), `quoin soup promote --discharges` (quoin-soup soup.rs:353) and `quoin release snapshot` (release/snapshot.rs:34). Either list them or state the rule generally and keep AC-17 to the direct call sites. | spec/functional/FR-029-consume-the-quire-json-contract.md:59-66,97 | wrong-requirement |
| FND-003 | medium | Behavior bullet 2 makes only the engine's *coverage* error an exhaustive mapping. The engine also reports drops through `RelationReportError::DeclarationsDropped` (relation_report, design_join), which quoin today maps by string to `Error::DesignJoin` (QQ-1110) in flow_report.rs:36 and design_join.rs:32. `quoin dossier generate`, `check` and `export` reach `flow_report::compute`, so a drop there would surface as QQ-1110, against "never degrades". The spec should require every engine error carrying `DroppedDeclarations` to map to QQ-1022, and require the direct `registry.traceability()` sites (`coverage::test_traces` for quoin-suite-trace, `formal_rules` status and assurance_declarations, `evidence::compute_with`) to check completeness themselves. | spec/functional/FR-029-consume-the-quire-json-contract.md:54-58; rust/crates/quoin-quire/src/flow_report.rs:27-37; rust/crates/quoin-quire/src/design_join.rs:23-32; rust/crates/quoin-quire/src/coverage.rs:84,157; rust/crates/quoin-cli/src/formal_rules.rs:136,262 | missing-requirement |
| FND-004 | medium | No AC covers the case that matters most for "a drop never degrades into QQ-1020": every declaration dropped, for example the only module's `traceability:` section is unreadable. The model is then empty, `Registry::traceability()` returns `None`, and quoin's own pre-check (`coverage::walk` coverage.rs:84, `test_traces` :157, `evidence.rs:238`, `formal_rules.rs:136/262`) returns QQ-1020 before the engine is asked. AC-14 tests one drop among valid declarations, so the model is non-empty and this path is never exercised. Add an AC: a selection whose only traceability section is dropped refuses with QQ-1022, not QQ-1020. | spec/functional/FR-029-consume-the-quire-json-contract.md:94; rust/crates/quoin-quire/src/coverage.rs:84 | missing-requirement |
| FND-005 | low | `quoin_quire::assurance::build` (public API, tested in tests/capabilities.rs:367) tolerates an absent model by design and binds over the merged model, but the engine's `build_assurance_export` does not refuse on drops. The spec is silent on whether it refuses. It has no production caller: `quoin assurance` goes through `quire.coverage`, `analysis_evidence` and `formal_rules::assurance_declarations`, so listing `quoin assurance` is sound. State whether the library function refuses on a drop, or leave it out of scope explicitly. | rust/crates/quoin-quire/src/assurance.rs:61-73 | missing-requirement |
| FND-006 | low | The behaviour now depends on quire-rs FR-058, but FR-029's `relationships` and its `## Dependencies` Upstream line list only quire-rs FR-055 and FR-053. FR-058 appears only as a body link. | spec/functional/FR-029-consume-the-quire-json-contract.md:4-11,102 | wrong-requirement |

## Verdict

Not mergeable: FND-001 is high. The new text has the engine's kind and item reversed.

What holds:

- The code allocation is correct and the Refused class is correct.
- `quoin assurance` is correctly included, and the distinction from the absent-model tolerance holds: the command already refuses QQ-1020 through `quire.coverage`, and only the unused library `assurance::build` tolerates an absent model.
- `quoin completeness` is correctly excluded. It reads `traceability.vocabulary_coverage` from manifests itself, not from the merged model, and its unresolved-declaration path is FR-037-AC-2.
- `quoin validate` is correctly excluded. The engine reports the drop there as the `trace:declared-model-dropped` finding.
- There is no contradiction with FR-029-AC-1..13, FR-037 or FR-096.

## Examined

FR-029 Behavior and AC-14..18. Context read:

- quire-rs FR-058 ("Consumers of the model" and AC-45..47), `src/coverage.rs`, `src/corpus/relation_report.rs`, `src/traceability/prune.rs`, `src/registry.rs` and `TraceabilityModel::is_empty`.
- quoin-quire `coverage.rs`, `evidence.rs`, `design_matrix.rs`, `design_join.rs`, `flow_report.rs`, `assurance.rs` and `error.rs`.
- quoin-core `ops/quire/taxonomy.rs` and `dispatch.rs`.
- quoin-cli `matrix.rs`, `assurance.rs`, `formal_rules.rs`, `formal_rules_coverage.rs`, `dossier.rs`, `advise.rs`, `anomaly.rs`, `evidence.rs`, `evidence/audit.rs`, `class_obligations.rs`, `release/snapshot.rs` and `completeness.rs`.
- quoin-soup `soup.rs`, quoin-dossier `architecture.rs` and quoin-suite-trace `main.rs`.
