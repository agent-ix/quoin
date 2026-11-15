---
id: SR-20261115183527
title: "Gap analysis — PLAT-1437 PR #933 SRS and architecture generators"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-192-dossier-srs-content.md, spec/functional/FR-193-dossier-architecture-content.md, spec/functional/FR-191-dossier-command.md, rust/crates/quoin-dossier, rust/crates/quoin-quire; PLAT-1437; PR #933"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-192"
    type: references
  - target: "ix://agent-ix/quoin/FR-193"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

## Summary

Gap analysis of quoin#933 at `ba4e6675`. Quoin was unavailable (blocked by the reviewer sandbox), so this is the manual criterion-to-test check: a grep index of `#[trace]` tags, not the computed matrix. 

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | coverage | None of FR-192-AC-1..AC-6 is tagged; no test calls `SrsGenerator::generate` |
| FND-002 | high | coverage | None of FR-193-AC-1..AC-6 is tagged; no test calls `ArchitectureGenerator::generate`; AC-2..AC-6 cannot pass as written |
| FND-003 | medium | trace | Three tests carry bare `FR-192` / `FR-193` tags while asserting only constants and a helper |
| FND-004 | medium | coverage | `quoin_quire::systems::load` / `Systems` and the six systems re-exports have no test and no caller |

## Verdict

**FAIL.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | None of FR-192-AC-1..AC-6 is tagged by a test and no test calls SrsGenerator::generate; ordering, unassigned criticality, the criticality_unassigned / no_acceptance_criteria / no_requirements gaps and StR Validation Criteria are unverified. | rust/crates/quoin-dossier/src/srs.rs:200-230 |
| FND-002 | high | None of FR-193-AC-1..AC-6 is tagged by a test and no test calls ArchitectureGenerator::generate; AC-2..AC-6 cannot pass with the current code and AC-1 fails for an uncovered requirement (code-review FND-002..FND-006). | rust/crates/quoin-dossier/src/architecture.rs:215-234 |
| FND-003 | medium | srs_generator_has_correct_spec, markdown_escape_replaces_pipe_and_newline and architecture_generator_has_correct_spec carry bare FR-192 / FR-193 tags but assert only DocumentSpec constants and a helper, so the requirements look backed in a trace view when no criterion is. | rust/crates/quoin-dossier/src/srs.rs:209, 220 |
| FND-004 | medium | quoin_quire::systems::load and Systems are new public API with no test and no caller; the PartRecord / PortRecord / ConnectionRecord / AllocationRecord / ConnectionDirection / PortDirection re-exports are unused in the workspace. | rust/crates/quoin-quire/src/systems.rs:13-40 |

## Reviewer data

```yaml
clean: false
scope:
  - {id: FR-192-AC-1, path: "spec/functional/FR-192-dossier-srs-content.md", role: examined, excerpt: "A fixture with three FRs, the third with no `criticality`, renders three sections in dossier order, and the third shows `unassigned` with a `criticality_unassigned` gap"}
  - {id: FR-192-AC-2, path: "spec/functional/FR-192-dossier-srs-content.md", role: examined, excerpt: "A requirement with zero criteria produces a `no_acceptance_criteria` gap naming it and no `unbacked_criterion` gap"}
  - {id: FR-192-AC-3, path: "spec/functional/FR-192-dossier-srs-content.md", role: examined, excerpt: "Absent criticality never renders a level: the section holds no level word for that requirement"}
  - {id: FR-192-AC-4, path: "spec/functional/FR-192-dossier-srs-content.md", role: examined, excerpt: "An empty scope renders no requirement section, a `no_requirements` gap and status `incomplete`"}
  - {id: FR-192-AC-5, path: "spec/functional/FR-192-dossier-srs-content.md", role: examined, excerpt: "Every criterion id and text of the fixture appears under its requirement"}
  - {id: FR-192-AC-6, path: "spec/functional/FR-192-dossier-srs-content.md", role: examined, excerpt: "A fixture StR with a Validation Criteria table lists each validation criterion id and text under the StR and records no `no_acceptance_criteria` gap for it"}
  - {id: FR-193-AC-1, path: "spec/functional/FR-193-dossier-architecture-content.md", role: examined, excerpt: "A fixture with two parts, one connection, one ADR and one requirement with no design element lists all elements and a `no_design_element` gap naming that requirement"}
  - {id: FR-193-AC-2, path: "spec/functional/FR-193-dossier-architecture-content.md", role: examined, excerpt: "A dangling design reference is listed and produces a `dangling_design_reference` gap"}
  - {id: FR-193-AC-3, path: "spec/functional/FR-193-dossier-architecture-content.md", role: examined, excerpt: "A module set declaring no `design_relations` yields status `incomplete` with a `design_relations_unavailable` gap, and `generate` does not exit 0"}
  - {id: FR-193-AC-4, path: "spec/functional/FR-193-dossier-architecture-content.md", role: examined, excerpt: "A part with no allocated requirement produces a `no_allocation` gap"}
  - {id: FR-193-AC-5, path: "spec/functional/FR-193-dossier-architecture-content.md", role: examined, excerpt: "A connection shows its flow direction and both ends"}
  - {id: FR-193-AC-6, path: "spec/functional/FR-193-dossier-architecture-content.md", role: examined, excerpt: "A wrong-kind design element under an `outgoing` relation is listed and produces a `design_kind_mismatch` gap naming the requirement and the element"}
  - {id: FR-191, path: "spec/functional/FR-191-dossier-command.md", role: context_only, excerpt: "the_shipped_registry_holds_srs_and_architecture asserts registry order srs, architecture"}
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-192
    related_ids: [FR-192-AC-1, FR-192-AC-2, FR-192-AC-3, FR-192-AC-4, FR-192-AC-5, FR-192-AC-6]
    path: rust/crates/quoin-dossier/src/srs.rs
    lines: "200-230"
    excerpt: |-
      When `quoin dossier generate` selects the SRS (`--doc srs`, document id `DOS-SRS`), the SRS SHALL list every requirement document in scope (StR, FR, NFR) in dossier order (FR-191) with its id, title, statement, criteria (id and text) and criticality as recorded.
    finding: "None of FR-192-AC-1..AC-6 is tagged by a test and no test calls SrsGenerator::generate; ordering, unassigned criticality, the criticality_unassigned / no_acceptance_criteria / no_requirements gaps and StR Validation Criteria are unverified."
  - fnd: FND-002
    severity: high
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-193
    related_ids: [FR-193-AC-1, FR-193-AC-2, FR-193-AC-3, FR-193-AC-4, FR-193-AC-5, FR-193-AC-6]
    path: rust/crates/quoin-dossier/src/architecture.rs
    lines: "215-234"
    excerpt: |-
      The architecture document (`--doc architecture`, document id `DOS-ARCH`) SHALL list parts, their ports, connections (with flow direction and ends), allocations and ADRs, and for each part the requirements allocated to or realized by it, from the quire-rs design join (quire-rs FR-086).
    finding: "None of FR-193-AC-1..AC-6 is tagged by a test and no test calls ArchitectureGenerator::generate; AC-2..AC-6 cannot pass with the current code and AC-1 fails for an uncovered requirement (code-review FND-002..FND-006)."
  - fnd: FND-003
    severity: medium
    confidence: high
    method: gap-analysis
    check: trace
    artifact_id: rust/crates/quoin-dossier/src/srs.rs
    related_ids: [FR-192, FR-193, rust/crates/quoin-dossier/src/architecture.rs]
    path: rust/crates/quoin-dossier/src/srs.rs
    lines: "209, 220"
    excerpt: |-
      #[trace("FR-192")]
      #[test]
      fn srs_generator_has_correct_spec() {
    test_id: srs_generator_has_correct_spec
    claimed_ac: FR-192
    correct_ac: none
    finding: "srs_generator_has_correct_spec, markdown_escape_replaces_pipe_and_newline and architecture_generator_has_correct_spec carry bare FR-192 / FR-193 tags but assert only DocumentSpec constants and a helper, so the requirements look backed in a trace view when no criterion is."
  - fnd: FND-004
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: rust/crates/quoin-quire/src/systems.rs
    related_ids: [rust/crates/quoin-quire/src/lib.rs, FR-193]
    path: rust/crates/quoin-quire/src/systems.rs
    lines: "13-40"
    excerpt: |-
      pub fn load(_request: &Request) -> Result<Systems> {
    finding: "quoin_quire::systems::load and Systems are new public API with no test and no caller; the PartRecord / PortRecord / ConnectionRecord / AllocationRecord / ConnectionDirection / PortDirection re-exports are unused in the workspace."
bindings:
  - {test_id: srs_generator_has_correct_spec, ac_id: FR-192, trace: wrong}
  - {test_id: markdown_escape_replaces_pipe_and_newline, ac_id: FR-192, trace: wrong}
  - {test_id: architecture_generator_has_correct_spec, ac_id: FR-193, trace: wrong}
  - {test_id: the_shipped_registry_holds_srs_and_architecture, ac_id: FR-191, trace: correct}
```

## Dispositions

All original findings were fixed after the two coder fix rounds. Criterion tests and fixture-backed systems tests are present; the ignored real-tool export test passed. The aggregate gate remains for the code lead.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | SRS tests cover FR-192-AC-1 through AC-6 with criterion tags and behavior assertions. |
| FND-002 | fixed | Architecture tests cover FR-193-AC-1 through AC-6 with criterion tags and behavior assertions. |
| FND-003 | fixed | Tests use criterion-level tags on behavior assertions. |
| FND-004 | fixed | `systems::load` has fixture-backed tests and a caller in the architecture generator. |
