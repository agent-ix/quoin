// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shared CLI fixture for a valid required relation beside a dropped relation.

pub(crate) const MODULE: &str = r"name: quoin-cli-dropped-relation-fixture
version: 0.1.0
artifact_types:
  - name: Requirement
  - name: DesignElement
  - name: Connection
  - name: Classification
edge_types:
  realizes: { description: realizes, category: traceability }
  carries: { description: carries, category: dataflow }
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: Requirement
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
  required_relations:
    - name: requirement-has-design
      from: Requirement
      edges: [realizes]
      to: [DesignElement]
      direction: outgoing
      check: requirement-without-design
    - name: connection-carries-classification
      from: Connection
      edges: [carries]
      to: [Classification]
      direction: outgoing
      check: connection-without-classification
      record_keys: [regulatedCategories]
  design_relations: [requirement-has-design]
";

pub(crate) const SPEC: &str = "---\nid: FR-001\ntype: Requirement\ntitle: Requirement\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n| FR-001-AC-1 | The requirement has a design entry. | Test (TC-001) |\n";
