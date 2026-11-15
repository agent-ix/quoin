---
id: SR-20261115183526
title: "Code review (Rust lane) — PLAT-1437 PR #933 SRS and architecture generators"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-dossier/src/srs.rs, rust/crates/quoin-dossier/src/architecture.rs, rust/crates/quoin-dossier/src/lib.rs, rust/crates/quoin-quire/src/systems.rs, rust/crates/quoin-quire/src/lib.rs, rust/crates/quoin-dossier/Cargo.toml, rust/crates/quoin-assurance/src/matrix/render.rs, rust/crates/quoin-dossier/src/engine.rs; PLAT-1437; PR #933"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-192"
    type: references
  - target: "ix://agent-ix/quoin/FR-193"
    type: references
---

## Summary

Code review (rust-review folded in) of quoin#933 at `ba4e6675`. Gates were not run: cargo, quoin and quire were blocked by the reviewer sandbox. 

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | code-bug | `quoin_quire::systems::load` is a stub: it ignores its request and always returns empty parts/ports/connections/allocations |
| FND-002 | high | soundness | Architecture never shows ports or connections: the Ports cell is blank and Connections is always `None.` (FR-193-AC-5) |
| FND-003 | high | code-bug | `design.is_some()` treats an `uncovered` join block as designed, so an uncovered requirement gets no `no_design_element` gap |
| FND-004 | high | soundness | `dangling` / `kind_mismatch` are never read; no `dangling_design_reference` or `design_kind_mismatch` gap; both are dropped (AC-2, AC-6) |
| FND-005 | high | soundness | No `design_relations_unavailable` gap; with no design relations every requirement becomes `no_design_element` (AC-3) |
| FND-006 | medium | soundness | No `no_allocation` gap for a part with no allocated requirement (AC-4) |
| FND-007 | medium | code-bug | SRS ignores `unreadable`, `malformed` and `contract: Violated`, so a malformed requirement is silently left out |
| FND-008 | medium | code-bug | Requirement ids are taken only from `entry.id`; entries keyed by criteria may vanish from the architecture document (not confirmed against quire-rs) |
| FND-009 | medium | other | Likely `-D warnings` failures: `unreachable_pub`, `single_char_add_str`, `redundant_closure_for_method_calls` (read, not run) |
| FND-010 | low | code-bug | SRS heading `## {id}: {title}` is not escaped; a newline in a title injects body text |
| FND-011 | low | other | `markdown_escape` duplicated in srs.rs and architecture.rs |

## Verdict

**FAIL.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | quoin_quire::systems::load ignores its Request and always returns four empty vectors: a public API that reports no parts, ports, connections or allocations for every corpus. FR-193 (Upstream (open)) makes exposing these records PLAT-1437's work. | rust/crates/quoin-quire/src/systems.rs:26-40 |
| FND-002 | high | The architecture document never shows ports or connections: the Ports cell is always blank and ## Connections is always the literal 'None.'. FR-193-AC-5 cannot pass for any input. | rust/crates/quoin-dossier/src/architecture.rs:123-135 |
| FND-003 | high | A requirement counts as designed when entry.design.is_some(), but the join returns blocks with state 'uncovered' and empty elements. An uncovered requirement therefore gets no no_design_element gap, against the FR-193 rule for the uncovered state. | rust/crates/quoin-dossier/src/architecture.rs:166-176 |
| FND-004 | high | The dangling and kind_mismatch lists of each design block are never read. No dangling_design_reference or design_kind_mismatch gap is produced, and the references and elements are dropped, against 'A dangling design reference or a kind mismatch SHALL NOT be dropped'. | rust/crates/quoin-dossier/src/architecture.rs:83-100 |
| FND-005 | high | No design_relations_unavailable gap is produced. With no design relations every entry's design is None, so every requirement is reported as no_design_element instead of the single cause the AC requires. | rust/crates/quoin-dossier/src/architecture.rs:166-195 |
| FND-006 | medium | No no_allocation gap is ever produced. Every element in ## Parts comes from a join entry that names a requirement, so a part with no allocation cannot be represented from this input. | rust/crates/quoin-dossier/src/architecture.rs:79-121 |
| FND-007 | medium | The SRS ignores Corpus::unreadable, Corpus::malformed and Document::contract == Violated, so a malformed requirement is silently left out of a document that must list every requirement in scope, with no gap or source failure. | rust/crates/quoin-dossier/src/srs.rs:39-55 |
| FND-008 | medium | Requirement ids are taken only from entry.id. quoin-assurance matrix.rs derives them from criteria[].criterion.id when criteria are present, so entries with id None would vanish from both Parts and Requirements without a design element. Not confirmed against quire-rs source. | rust/crates/quoin-dossier/src/architecture.rs:90-96, 168-175 |
| FND-009 | medium | Likely -D warnings failures under clippy::all + pedantic and the workspace unreachable_pub lint (read, not run): pub structs in private modules, body.push_str("\n") (single_char_add_str, four sites) and .map(\|path\| ModuleRoot::open(path)) (redundant_closure_for_method_calls, two sites). | rust/crates/quoin-dossier/src/srs.rs:15, 32, 113 |
| FND-010 | low | The SRS requirement heading is not escaped; a frontmatter title containing a newline ends the heading and injects body text. | rust/crates/quoin-dossier/src/srs.rs:69 |
| FND-011 | low | markdown_escape is defined twice with identical bodies in srs.rs and architecture.rs; only the srs.rs copy is tested. | rust/crates/quoin-dossier/src/architecture.rs:208-210 |

## Reviewer data

```yaml
clean: false
scope:
  - {id: FR-192, path: "spec/functional/FR-192-dossier-srs-content.md", role: examined, excerpt: "When `quoin dossier generate` selects the SRS (`--doc srs`, document id `DOS-SRS`), the SRS SHALL list every requirement document in scope (StR, FR, NFR) in dossier order (FR-191) with its id, title, statement, criteria (id and text) and criticality as recorded."}
  - {id: FR-193, path: "spec/functional/FR-193-dossier-architecture-content.md", role: examined, excerpt: "The architecture document (`--doc architecture`, document id `DOS-ARCH`) SHALL list parts, their ports, connections (with flow direction and ends), allocations and ADRs, and for each part the requirements allocated to or realized by it, from the quire-rs design join (quire-rs FR-086)."}
  - {id: rust/crates/quoin-dossier/src/srs.rs, path: rust/crates/quoin-dossier/src/srs.rs, role: examined, excerpt: "SRS document generator (FR-192, PLAT-1437)."}
  - {id: rust/crates/quoin-dossier/src/architecture.rs, path: rust/crates/quoin-dossier/src/architecture.rs, role: examined, excerpt: "Architecture document generator (FR-193, PLAT-1437)."}
  - {id: rust/crates/quoin-dossier/src/lib.rs, path: rust/crates/quoin-dossier/src/lib.rs, role: examined, excerpt: "generators.push(Box::new(srs::SrsGenerator)); generators.push(Box::new(architecture::ArchitectureGenerator));"}
  - {id: rust/crates/quoin-quire/src/systems.rs, path: rust/crates/quoin-quire/src/systems.rs, role: examined, excerpt: "pub fn load(_request: &Request) -> Result<Systems>"}
  - {id: rust/crates/quoin-quire/src/lib.rs, path: rust/crates/quoin-quire/src/lib.rs, role: examined, excerpt: "pub use quire_rs::semantic::systems::{AllocationRecord, ConnectionDirection, ConnectionRecord, PartRecord, PortDirection, PortRecord};"}
  - {id: rust/crates/quoin-dossier/Cargo.toml, path: rust/crates/quoin-dossier/Cargo.toml, role: examined, excerpt: "quoin-quire = { workspace = true }"}
  - {id: rust/crates/quoin-assurance/src/matrix/render.rs, path: rust/crates/quoin-assurance/src/matrix/render.rs, role: context_only, excerpt: "for block in blocks_with_state(requirement.design.as_ref(), \"uncovered\") {"}
  - {id: rust/crates/quoin-dossier/src/engine.rs, path: rust/crates/quoin-dossier/src/engine.rs, role: context_only, excerpt: "let text = compose(&spec, &inputs.version, inputs.as_of.as_deref(), &generated.body, &generated.gaps);"}
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: rust/crates/quoin-quire/src/systems.rs
    related_ids: [FR-193]
    path: rust/crates/quoin-quire/src/systems.rs
    lines: "26-40"
    excerpt: |-
      pub fn load(_request: &Request) -> Result<Systems> {
          // Systems records (part, port, connection, allocation per FR-075) are
          // extracted from document body tables during semantic validation.
          // The architecture dossier uses design elements from design_join instead.
          Ok(Systems {
              parts: Vec::new(),
              ports: Vec::new(),
              connections: Vec::new(),
              allocations: Vec::new(),
          })
      }
    finding: "quoin_quire::systems::load ignores its Request and always returns four empty vectors: a public API that reports no parts, ports, connections or allocations for every corpus. FR-193 (Upstream (open)) makes exposing these records PLAT-1437's work."
  - fnd: FND-002
    severity: high
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FR-193-AC-5
    related_ids: [FR-193, rust/crates/quoin-dossier/src/architecture.rs]
    path: rust/crates/quoin-dossier/src/architecture.rs
    lines: "123-135"
    excerpt: |-
      A connection shows its flow direction and both ends
    finding: "The architecture document never shows ports or connections: the Ports cell is always blank and ## Connections is always the literal 'None.'. FR-193-AC-5 cannot pass for any input."
  - fnd: FND-003
    severity: high
    confidence: medium
    method: code-review
    check: code-bug
    artifact_id: rust/crates/quoin-dossier/src/architecture.rs
    related_ids: [FR-193, FR-193-AC-1]
    path: rust/crates/quoin-dossier/src/architecture.rs
    lines: "166-176"
    excerpt: |-
      for entry in &design_outcome.design_matrix {
          if let Some(req_id) = &entry.id {
              all_requirements.insert(req_id.clone());
              if entry.design.is_some() {
                  requirements_with_design.insert(req_id.clone());
              }
          }
      }
    finding: "A requirement counts as designed when entry.design.is_some(), but the join returns blocks with state 'uncovered' and empty elements. An uncovered requirement therefore gets no no_design_element gap, against the FR-193 rule for the uncovered state."
  - fnd: FND-004
    severity: high
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FR-193-AC-2
    related_ids: [FR-193, FR-193-AC-6, rust/crates/quoin-dossier/src/architecture.rs]
    path: rust/crates/quoin-dossier/src/architecture.rs
    lines: "83-100"
    excerpt: |-
      A dangling design reference is listed and produces a `dangling_design_reference` gap
    finding: "The dangling and kind_mismatch lists of each design block are never read. No dangling_design_reference or design_kind_mismatch gap is produced, and the references and elements are dropped, against 'A dangling design reference or a kind mismatch SHALL NOT be dropped'."
  - fnd: FND-005
    severity: high
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FR-193-AC-3
    related_ids: [FR-193, rust/crates/quoin-dossier/src/architecture.rs]
    path: rust/crates/quoin-dossier/src/architecture.rs
    lines: "166-195"
    excerpt: |-
      A module set declaring no `design_relations` yields status `incomplete` with a `design_relations_unavailable` gap, and `generate` does not exit 0
    finding: "No design_relations_unavailable gap is produced. With no design relations every entry's design is None, so every requirement is reported as no_design_element instead of the single cause the AC requires."
  - fnd: FND-006
    severity: medium
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FR-193-AC-4
    related_ids: [FR-193, rust/crates/quoin-dossier/src/architecture.rs]
    path: rust/crates/quoin-dossier/src/architecture.rs
    lines: "79-121"
    excerpt: |-
      A part with no allocated requirement produces a `no_allocation` gap
    finding: "No no_allocation gap is ever produced. Every element in ## Parts comes from a join entry that names a requirement, so a part with no allocation cannot be represented from this input."
  - fnd: FND-007
    severity: medium
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: rust/crates/quoin-dossier/src/srs.rs
    related_ids: [FR-192]
    path: rust/crates/quoin-dossier/src/srs.rs
    lines: "39-55"
    excerpt: |-
      let mut requirements: Vec<_> = corpus
          .documents
          .iter()
          .filter(|doc| ["FR", "NFR", "StR"].contains(&doc.kind.as_str()))
          .collect();
    finding: "The SRS ignores Corpus::unreadable, Corpus::malformed and Document::contract == Violated, so a malformed requirement is silently left out of a document that must list every requirement in scope, with no gap or source failure."
  - fnd: FND-008
    severity: medium
    confidence: low
    method: code-review
    check: code-bug
    artifact_id: rust/crates/quoin-dossier/src/architecture.rs
    related_ids: [FR-193]
    path: rust/crates/quoin-dossier/src/architecture.rs
    lines: "90-96, 168-175"
    excerpt: |-
      if let Some(req_id) = &entry.id {
          element_requirements
              .entry(element.id.clone())
              .or_default()
              .push(req_id.clone());
      }
    finding: "Requirement ids are taken only from entry.id. quoin-assurance matrix.rs derives them from criteria[].criterion.id when criteria are present, so entries with id None would vanish from both Parts and Requirements without a design element. Not confirmed against quire-rs source."
  - fnd: FND-009
    severity: medium
    confidence: medium
    method: code-review
    check: other
    artifact_id: rust/crates/quoin-dossier/src/srs.rs
    related_ids: [rust/crates/quoin-dossier/src/architecture.rs]
    path: rust/crates/quoin-dossier/src/srs.rs
    lines: "15, 32, 113"
    excerpt: |-
      pub struct SrsGenerator;
    finding: "Likely -D warnings failures under clippy::all + pedantic and the workspace unreachable_pub lint (read, not run): pub structs in private modules, body.push_str(\"\\n\") (single_char_add_str, four sites) and .map(|path| ModuleRoot::open(path)) (redundant_closure_for_method_calls, two sites)."
  - fnd: FND-010
    severity: low
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: rust/crates/quoin-dossier/src/srs.rs
    related_ids: [FR-192]
    path: rust/crates/quoin-dossier/src/srs.rs
    lines: "69"
    excerpt: |-
      body.push_str(&format!("## {}: {}\n\n", req.id, title_of(req)));
    finding: "The SRS requirement heading is not escaped; a frontmatter title containing a newline ends the heading and injects body text."
  - fnd: FND-011
    severity: low
    confidence: high
    method: code-review
    check: other
    artifact_id: rust/crates/quoin-dossier/src/architecture.rs
    related_ids: [rust/crates/quoin-dossier/src/srs.rs]
    path: rust/crates/quoin-dossier/src/architecture.rs
    lines: "208-210"
    excerpt: |-
      fn markdown_escape(text: &str) -> String {
          text.replace('|', "\\|").replace('\n', " ")
      }
    finding: "markdown_escape is defined twice with identical bodies in srs.rs and architecture.rs; only the srs.rs copy is tested."
bindings: []
```
