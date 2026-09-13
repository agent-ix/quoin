// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The bundle, obligation and case-input fixtures the FR-040 suites share.
//!
//! Included by `case_graph.rs`, `case_rendering.rs` and
//! `case_assessment_context.rs`; the preamble below arrived with them, from
//! the single `tests/case.rs` they were split out of.
//!
//! FR-040's acceptance criteria, against the public API (quoin#447).
//!
//! # What this file is carrying forward
//!
//! `quoin_assurance::case` and `quoin_assurance::render` arrived with no test
//! of their own: the difftest compares them against the retained
//! implementation, which says the two agree and says nothing about whether
//! either is right. The criteria below are `tests/assurance.test.ts`'s, which
//! is the source of truth for WHAT is asserted here.
//!
//! One of FR-040's criteria is not restated here and is not missing:
//! **FR-040-AC-7** (`requirementOf` derives an obligation's owner from its id)
//! is already held by the unit tests beside
//! [`quoin_assurance::requirement_of`], which reach five spellings this file
//! would only repeat.
//!
//! Three further criteria are carried by the *other* two retained suites that
//! render an assurance case — `tests/independence.test.ts` (FR-094-AC-8) and
//! `tests/trust-decision.test.ts` (FR-093-AC-7). They are restated here
//! because [`render_case`] is where the behaviour lives; the assessment
//! *derivations* those suites also cover are not this crate's code.
//!
//! # Why the fixtures are written here and the expectations are not derived
//!
//! FR-101 names the self-fixture tautology. The documents below are
//! hand-written frontmatter, exactly as the retained test wrote them — that is
//! the input shape, and there is no live corpus to capture one from in CI.
//! What makes these non-tautological is the other half: every expectation is
//! an independently stated literal — whole [`CaseNode`]s, the exact `because`
//! and `reason` sentences, and the rendered markdown block by block — never a
//! second call to the code under test.

use quoin_assurance::case::CaseInput;
use quoin_finding_types::Finding;
use quoin_quire_types::Obligation;

/// A bundle document's frontmatter, in the shape `BundleDocument` serialises.
pub(crate) fn doc(
    id: &str,
    kind: &str,
    title: &str,
    parents: &[(&str, &str)],
) -> serde_json::Value {
    let relationships: Vec<serde_json::Value> = parents
        .iter()
        .map(|(target, edge)| {
            serde_json::json!({
                "target": format!("ix://agent-ix/quoin/{target}"),
                "type": edge,
            })
        })
        .collect();
    serde_json::json!({
        "path": format!("spec/{id}.md"),
        "frontmatter": {
            "id": id,
            "type": kind,
            "title": title,
            "relationships": relationships,
        },
        "body": "",
    })
}

pub(crate) fn obligation(id: &str, statement: &str) -> Obligation {
    Obligation {
        id: id.to_owned(),
        statement: statement.to_owned(),
    }
}

pub(crate) fn input(
    documents: Vec<serde_json::Value>,
    obligations: Vec<Obligation>,
    findings: Vec<Finding>,
) -> CaseInput {
    CaseInput {
        documents,
        obligations,
        findings,
        claim_types: None,
        unreadable: None,
        producer_trust: None,
        evidence_independence: None,
    }
}

/// `StR-001 ← FR-001` (`traces_to`), the retained test's own bundle.
pub(crate) fn bundle() -> Vec<serde_json::Value> {
    vec![
        doc("StR-001", "StR", "The system is usable", &[]),
        doc(
            "FR-001",
            "FR",
            "Parse the input",
            &[("StR-001", "traces_to")],
        ),
    ]
}
