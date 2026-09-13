// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The assessment sections a case renders as context, against the public API
//! (quoin#447).
//!
//! Evidence independence (FR-094) and evidence-producer reliance (FR-093):
//! both are appended to the case and neither moves claim support. The fixtures
//! are in `common/fixtures.rs`, which also carries the preamble this suite was
//! split out of.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_assurance::case::{NodeStatus, build_case};

#[path = "common/fixtures.rs"]
mod fixtures;
#[path = "common/render.rs"]
mod render_hop;

use crate::fixtures::{bundle, input, obligation};
use crate::render_hop::render;

/// A satisfied independence assessment, in the shape the evidence store writes.
fn independence() -> serde_json::Value {
    serde_json::json!({
        "profile": "AP-001",
        "requirement": "IR-1",
        "obligation": "FR-001-AC-1",
        "status": "satisfied",
        "summary": "two suites, distinct toolchains",
        "satisfiedBy": ["SUITE-A", "SUITE-B"],
        "dimensions": [
            {
                "dimension": "implementation-toolchain",
                "values": ["rustc-1.98", "node-22"],
                "missingSuites": []
            },
            {
                "dimension": "author",
                "values": [],
                "missingSuites": ["SUITE-B"]
            }
        ]
    })
}

/// Trace: FR-094-AC-8
/// Provenance: agent-ix/quoin#447, tests/independence.test.ts
#[test]
fn tc_447_415_a_satisfied_independence_assessment_renders_as_context_not_as_claim_support() {
    let mut with_assessment = input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    );
    with_assessment.evidence_independence = Some(vec![independence()]);
    let case = build_case(&with_assessment);
    let plain = build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    ));

    // Claim support is UNCHANGED — not merely "still supported", but the same
    // tree the case has without any assessment at all.
    assert_eq!(case.claims, plain.claims);
    assert_eq!(case.claims[0].status, NodeStatus::Supported);

    let rendered = render(&case);
    // The section is appended to the case, so the whole of it is the tail —
    // which also asserts that nothing of the case above it moved.
    assert!(
        rendered.starts_with(&render(&plain)),
        "the case above the section must not move:\n{rendered}"
    );
    assert!(rendered.ends_with(
        "## Evidence independence\n\
         \n\
         These are profile-selected relationship checks; they do not make a claim supported by themselves.\n\
         \n\
         - **AP-001 / IR-1 / FR-001-AC-1** — satisfied\n\
         \x20 - two suites, distinct toolchains\n\
         \x20 - implementation-toolchain: rustc-1.98, node-22\n\
         \x20 - author: no stated values; missing on SUITE-B\n"
    ), "{rendered}");
    // The criterion's own four substrings, as the retained test states them.
    assert!(rendered.contains("## Evidence independence"));
    assert!(rendered.contains("AP-001 / IR-1 / FR-001-AC-1"));
    assert!(rendered.contains("implementation-toolchain"));
    assert!(rendered.contains("do not make a claim supported"));
}

/// Trace: FR-094-AC-7, FR-094-AC-8
/// Provenance: agent-ix/quoin#447, tests/independence.test.ts
#[test]
fn tc_447_416_an_empty_independence_list_emits_no_heading_and_an_absent_one_is_legal() {
    // `(x?.length ?? 0) > 0` in the retained source: an absent list and an
    // EMPTY list both skip the section. An `is_some()` branch would emit a
    // heading with nothing under it, which reads as "the check ran and found
    // no dimensions" rather than "no check was selected".
    let mut empty = input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    );
    empty.evidence_independence = Some(vec![]);
    let with_empty = build_case(&empty);
    assert_eq!(with_empty.evidence_independence, Some(vec![]));
    assert!(!render(&with_empty).contains("## Evidence independence"));

    // Absent is legal, and stays absent in the payload: the key's presence is
    // what tells a consumer a policy was selected at all.
    let absent = build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    ));
    assert_eq!(absent.evidence_independence, None);
    let payload = serde_json::to_value(&absent).expect("it serialises");
    assert!(payload.get("evidenceIndependence").is_none(), "{payload}");
    assert!(!render(&absent).contains("## Evidence independence"));

    // The two render identically, which is the criterion stated as an
    // equality rather than as two absences.
    assert_eq!(render(&with_empty), render(&absent));
}

/// Trace: FR-093-AC-7
/// Provenance: agent-ix/quoin#447, tests/trust-decision.test.ts
#[test]
fn tc_447_417_an_invalidated_producer_trust_decision_renders_without_changing_claim_support() {
    let mut with_trust = input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    );
    with_trust.producer_trust = Some(vec![serde_json::json!({
        "id": "ETD-001",
        "useId": "advisory-review",
        "producer": "vitest",
        "status": "invalidated",
        "triggeredBy": ["producer-version"],
        "limitations": [],
        "permittedDecisions": ["release"],
        "owner": "qa"
    })]);
    let case = build_case(&with_trust);
    let plain = build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    ));

    assert_eq!(case.producer_trust.len(), 1);
    assert_eq!(
        case.producer_trust[0]
            .get("status")
            .and_then(serde_json::Value::as_str),
        Some("invalidated")
    );
    // Context, never support: the same tree as a case with no trust decision.
    assert_eq!(case.claims, plain.claims);
    assert_eq!(case.claims[0].status, NodeStatus::Supported);

    let rendered = render(&case);
    assert!(
        rendered.contains("invalidated — revalidate: producer-version"),
        "{rendered}"
    );
    assert!(
        rendered.ends_with(
            "## Evidence-producer reliance\n\
         \n\
         These decisions are context for the case; they do not make a claim supported.\n\
         \n\
         - **ETD-001 / advisory-review** — invalidated — revalidate: producer-version\n"
        ),
        "{rendered}"
    );
    assert!(rendered.starts_with(&render(&plain)), "{rendered}");
}
