// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-040's graph construction criteria, against the public API (quoin#447).
//!
//! Claims down to evidence, open nodes and the ancestors they open,
//! undeveloped claims, unreached requirements, refinement cycles and
//! claim-type matching. The fixtures are in `common/fixtures.rs`, which also
//! carries the preamble this suite was split out of.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_assurance::case::{CaseNode, NodeKind, NodeStatus, build_case};
use quoin_finding_types::Finding;

#[path = "common/fixtures.rs"]
mod fixtures;

use crate::fixtures::{bundle, doc, input, obligation};

fn finding(obligation_id: &str, kind: &str) -> Finding {
    Finding {
        obligation: obligation_id.to_owned(),
        kind: kind.to_owned(),
        summary: format!("{obligation_id} has a problem"),
        other: std::collections::BTreeMap::new(),
    }
}

/// Trace: FR-040-AC-1
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_400_argues_from_the_claim_down_to_its_evidence() {
    let result = build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    ));

    assert_eq!(result.claims.len(), 1);
    let claim = &result.claims[0];
    assert_eq!(claim.id, "StR-001");
    assert_eq!(claim.kind, NodeKind::Goal);
    assert_eq!(claim.statement, "The system is usable");
    assert_eq!(claim.status, NodeStatus::Supported);
    assert_eq!(claim.because, None);

    assert_eq!(claim.children.len(), 1);
    let requirement = &claim.children[0];
    assert_eq!(requirement.id, "FR-001");
    assert_eq!(requirement.kind, NodeKind::Goal);
    assert_eq!(requirement.status, NodeStatus::Supported);

    // The leaf stated whole rather than field by field: the criterion is the
    // node, and a `matchObject` over three keys would pass with a stray child
    // or a stray `because` hanging off it.
    assert_eq!(
        requirement.children,
        vec![CaseNode {
            id: "FR-001-AC-1".to_owned(),
            kind: NodeKind::Solution,
            statement: "It holds.".to_owned(),
            status: NodeStatus::Supported,
            because: None,
            children: Vec::new(),
        }]
    );
}

/// Trace: FR-040-AC-2
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_401_an_obligation_with_a_finding_stays_in_the_tree_as_an_open_node() {
    // The point of the whole view. Dropping it produces a case that reads as
    // complete, which is the failure mode an assurance case invites most.
    let result = build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![finding("FR-001-AC-1", "undischarged")],
    ));

    let leaf = &result.claims[0].children[0].children[0];
    assert_eq!(leaf.id, "FR-001-AC-1");
    assert_eq!(leaf.status, NodeStatus::Open);
    // `{kind}: {summary}`, stated as the literal the reader sees. The retained
    // test asserts only that the KIND appears; the whole sentence is what the
    // node actually carries, and an auditor reads it verbatim.
    assert_eq!(
        leaf.because.as_deref(),
        Some("undischarged: FR-001-AC-1 has a problem")
    );
}

/// Trace: FR-040-AC-3
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_402_one_open_leaf_opens_every_ancestor_up_to_the_claim() {
    let result = build_case(&input(
        bundle(),
        vec![
            obligation("FR-001-AC-1", "It holds."),
            obligation("FR-001-AC-2", "It holds."),
        ],
        vec![finding("FR-001-AC-2", "suspect-link")],
    ));

    let claim = &result.claims[0];
    assert_eq!(claim.status, NodeStatus::Open);
    assert_eq!(claim.children[0].status, NodeStatus::Open);

    // Open propagates; it does not spread. The sibling that carries no finding
    // is still supported, and an ancestor opened by "everything under me is
    // open" would be a different, weaker view.
    let leaves = &claim.children[0].children;
    assert_eq!(leaves[0].id, "FR-001-AC-1");
    assert_eq!(leaves[0].status, NodeStatus::Supported);
    assert_eq!(leaves[1].id, "FR-001-AC-2");
    assert_eq!(leaves[1].status, NodeStatus::Open);

    // And the opened ancestors say nothing false about WHY: `because` belongs
    // to the empty goal, not to the propagated one.
    assert_eq!(claim.because, None);
    assert_eq!(claim.children[0].because, None);
}

/// Trace: FR-040-AC-4, FR-040-CON-2
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_403_a_claim_nothing_traces_to_is_undeveloped_and_never_supported() {
    // A goal with no sub-goal and no evidence is open. Rendering it supported
    // would assure a claim on the strength of nobody having written anything
    // against it.
    let result = build_case(&input(
        vec![doc("StR-002", "StR", "Nothing argues for this", &[])],
        vec![],
        vec![],
    ));

    assert_eq!(
        result.claims,
        vec![CaseNode {
            id: "StR-002".to_owned(),
            kind: NodeKind::Goal,
            statement: "Nothing argues for this".to_owned(),
            status: NodeStatus::Open,
            because: Some("no sub-claim and no obligation traces to this claim".to_owned()),
            children: Vec::new(),
        }]
    );
    // The retained criterion is stated as a substring, and it is the half a
    // reader scans for.
    assert!(
        result.claims[0]
            .because
            .as_deref()
            .expect("the reason is visible")
            .contains("no sub-claim")
    );
}

/// Trace: FR-040-AC-5
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_404_a_requirement_no_claim_reaches_is_reported_rather_than_rendered_around() {
    // A case drawn only over the reachable half reads as complete. The orphan
    // has obligations, which is what makes its absence from the argument a gap
    // rather than a document nobody needed.
    let mut documents = bundle();
    documents.push(doc("FR-009", "FR", "Orphaned", &[]));
    let result = build_case(&input(
        documents,
        vec![
            obligation("FR-001-AC-1", "It holds."),
            obligation("FR-009-AC-1", "It holds."),
        ],
        vec![],
    ));

    assert_eq!(result.unreachable, ["FR-009"]);
    // The reached half is genuinely reached, not merely absent from the list.
    assert_eq!(result.claims[0].children[0].id, "FR-001");
}

/// Trace: FR-040-AC-11
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#170
#[test]
fn tc_447_405_a_requirement_that_traces_to_two_claims_appears_under_both() {
    // Sharing one visited-set across claims put it under the first only, and
    // the second reported "no sub-claim traces to this claim" — a FALSE
    // statement, in an assurance case, about the very edge its author wrote.
    // Cycle prevention and "already rendered somewhere" are different questions.
    let documents = vec![
        doc("StR-001", "StR", "One", &[]),
        doc("StR-002", "StR", "Two", &[]),
        doc(
            "FR-001",
            "FR",
            "Shared",
            &[("StR-001", "traces_to"), ("StR-002", "traces_to")],
        ),
    ];
    let result = build_case(&input(
        documents,
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    ));

    let children: Vec<Vec<&str>> = result
        .claims
        .iter()
        .map(|claim| claim.children.iter().map(|c| c.id.as_str()).collect())
        .collect();
    assert_eq!(children, [["FR-001"], ["FR-001"]]);
    assert!(result.claims.iter().all(|claim| claim.because.is_none()));

    // The second copy is a whole subtree, not a stub: the obligation hangs off
    // both, which is what "appears under both" has to mean for a reader.
    for claim in &result.claims {
        assert_eq!(claim.children[0].children[0].id, "FR-001-AC-1");
    }
}

/// Trace: FR-040-AC-12
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_406_a_refinement_cycle_terminates_without_dropping_a_shared_child() {
    // A refines B refines A. The recursion must terminate, and it must do so by
    // path and not by "seen anywhere", or the fix above regresses.
    let documents = vec![
        doc("StR-001", "StR", "Root", &[]),
        doc(
            "FR-001",
            "FR",
            "A",
            &[("StR-001", "traces_to"), ("FR-002", "refines")],
        ),
        doc("FR-002", "FR", "B", &[("FR-001", "refines")]),
    ];
    let result = build_case(&input(documents, vec![], vec![]));

    assert_eq!(result.claims.len(), 1);
    let serialised = serde_json::to_string(&result.claims[0]).expect("it serialises");
    assert!(serialised.len() < 4000, "{}", serialised.len());

    // Bounded, and bounded at the right place: the cycle is cut on the second
    // visit to `FR-001` on this path, not by refusing to descend at all.
    let claim = &result.claims[0];
    assert_eq!(claim.children[0].id, "FR-001");
    assert_eq!(claim.children[0].children[0].id, "FR-002");
    assert!(claim.children[0].children[0].children.is_empty());
}

/// Trace: FR-040-AC-6
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_407_the_claim_type_comes_from_the_caller_and_never_from_a_constant() {
    // A safety bundle argues from a declared hazard. The vocabulary is module
    // data, so a built-in `StR` would make this view useless to exactly the
    // bundles that need an assurance case most.
    let documents = vec![
        doc("HAZ-001", "hazard", "Uncommanded actuation", &[]),
        doc("FR-001", "FR", "Interlock", &[("HAZ-001", "mitigates")]),
    ];

    let with_default = build_case(&input(documents.clone(), vec![], vec![]));
    assert!(with_default.claims.is_empty());

    let mut hazard = input(documents, vec![], vec![]);
    hazard.claim_types = Some(vec!["hazard".to_owned()]);
    let with_hazard = build_case(&hazard);
    assert_eq!(with_hazard.claims.len(), 1);
    assert_eq!(with_hazard.claims[0].id, "HAZ-001");
    assert_eq!(with_hazard.claims[0].children[0].id, "FR-001");
    // `mitigates` is a downward edge like `traces_to`: the hazard bundle's own
    // vocabulary reaches the requirement, rather than the requirement being
    // reported unreachable under a claim that names it.
    assert!(with_hazard.unreachable.is_empty());
}

/// Trace: FR-040-AC-13
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#170
#[test]
fn tc_447_408_the_machine_readable_reason_is_present_exactly_when_nothing_is_a_claim() {
    // `--json` emits `build_case`'s result verbatim, so this field is what lets
    // a pipeline tell "the case is clean" from "nothing matched, so nothing
    // was argued" — `claims: []` alone reads the same both ways.
    let mut searched = input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    );
    searched.claim_types = Some(vec!["hazard".to_owned()]);
    let empty = build_case(&searched);

    assert!(empty.claims.is_empty());
    // The whole sentence, including the SEARCHED types as the caller spelled
    // them: the reader of the JSON needs to see that `hazard` argued nothing.
    assert_eq!(
        empty.reason.as_deref(),
        Some(
            "no document declares itself a top-level claim (searched claim types: hazard); declare one or pass --claim-type"
        )
    );

    // And it survives the payload, which is the only form a pipeline sees.
    let payload = serde_json::to_value(&empty).expect("it serialises");
    assert_eq!(
        payload.get("reason").and_then(serde_json::Value::as_str),
        empty.reason.as_deref()
    );

    // Two spellings, both echoed back verbatim rather than normalised, so a
    // caller can see which of its own arguments matched nothing.
    let mut two = input(bundle(), vec![], vec![]);
    two.claim_types = Some(vec!["hazard".to_owned(), "Threat".to_owned()]);
    assert_eq!(
        build_case(&two).reason.as_deref(),
        Some(
            "no document declares itself a top-level claim (searched claim types: hazard, Threat); declare one or pass --claim-type"
        )
    );

    // ABSENT — not empty-string — on a case with claims, so the presence of
    // the key is itself the signal.
    let non_empty = build_case(&input(bundle(), vec![], vec![]));
    assert_eq!(non_empty.claims.len(), 1);
    assert_eq!(non_empty.reason, None);
    let payload = serde_json::to_value(&non_empty).expect("it serialises");
    assert!(
        payload.get("reason").is_none(),
        "the key itself must be absent: {payload}"
    );
}

/// Trace: FR-040-AC-14
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#170
#[test]
fn tc_447_409_claim_type_matching_is_case_insensitive_in_both_directions() {
    // `str`, `STR` and `Hazard` all matched nothing under `===` and exited 0
    // with an empty case — silence indistinguishable from a clean corpus.
    let documents = vec![
        doc("HAZ-001", "hazard", "Uncommanded actuation", &[]),
        doc("FR-001", "FR", "Interlock", &[("HAZ-001", "mitigates")]),
    ];
    let mut upper = input(documents, vec![], vec![]);
    upper.claim_types = Some(vec!["Hazard".to_owned()]);
    let matched = build_case(&upper);
    let ids: Vec<&str> = matched.claims.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["HAZ-001"]);

    // The default matches an authored `str` too — the vocabulary is module
    // data, and casing is not part of what a claim type means.
    let lowercased = build_case(&input(
        vec![doc("StR-001", "str", "The system is usable", &[])],
        vec![],
        vec![],
    ));
    let ids: Vec<&str> = lowercased.claims.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["StR-001"]);
}
