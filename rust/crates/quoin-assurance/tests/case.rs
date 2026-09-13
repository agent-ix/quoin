// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

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

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_assurance::case::{AssuranceCase, CaseInput, CaseNode, NodeKind, NodeStatus, build_case};
use quoin_assurance::render::{RenderableCase, render_case};
use quoin_finding_types::Finding;
use quoin_quire_types::Obligation;

/// A bundle document's frontmatter, in the shape `BundleDocument` serialises.
fn doc(id: &str, kind: &str, title: &str, parents: &[(&str, &str)]) -> serde_json::Value {
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

fn obligation(id: &str, statement: &str) -> Obligation {
    Obligation {
        id: id.to_owned(),
        statement: statement.to_owned(),
    }
}

fn finding(obligation_id: &str, kind: &str) -> Finding {
    Finding {
        obligation: obligation_id.to_owned(),
        kind: kind.to_owned(),
        summary: format!("{obligation_id} has a problem"),
    }
}

fn input(
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
fn bundle() -> Vec<serde_json::Value> {
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

/// The built case as the renderer reads it.
///
/// `build_case` carries both assessment lists as opaque values; `render_case`
/// reads fourteen of their fields. The hop between the two type parameters is
/// the payload itself, which is how `quoin-core` does it — so this is the real
/// path a caller takes, not a test-only shortcut around the seam.
fn renderable(case: &AssuranceCase) -> RenderableCase {
    let payload = serde_json::to_value(case).expect("the case serialises");
    serde_json::from_value(payload).expect("the payload is a renderable case")
}

fn render(case: &AssuranceCase) -> String {
    render_case(&renderable(case))
}

/// The contents of the first mermaid fence, exclusive of the fences.
fn mermaid(rendered: &str) -> String {
    rendered
        .split("```mermaid")
        .nth(1)
        .expect("a mermaid block is rendered")
        .split("```")
        .next()
        .expect("the fence closes")
        .to_owned()
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

/// Trace: FR-040-AC-8
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_410_rendering_is_byte_identical_over_unchanged_inputs() {
    // A store view, never a hand-maintained document: a diff must mean the
    // evidence changed, not that somebody re-ran it.
    let first = render(&build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    )));
    let second = render(&build_case(&input(
        bundle(),
        vec![obligation("FR-001-AC-1", "It holds.")],
        vec![],
    )));
    assert_eq!(first, second);

    // The literal, not a re-derivation: the whole document for the retained
    // test's own bundle, heading by heading and blank line by blank line.
    assert_eq!(
        first,
        "# Assurance case\n\
         \n\
         1 claim(s), 0 with an open branch.\n\
         \n\
         ## ✅ StR-001 — The system is usable\n\
         \n\
         \x20 - ✅ FR-001 — Parse the input\n\
         \x20   - ✅ **FR-001-AC-1** — It holds.\n\
         \n\
         ```mermaid\n\
         flowchart TD\n\
         \x20 StR_001[\"StR-001: The system is usable\"]\n\
         \x20 FR_001[\"FR-001: Parse the input\"]\n\
         \x20 FR_001_AC_1([\"FR-001-AC-1: It holds.\"])\n\
         \x20 FR_001 --> FR_001_AC_1\n\
         \x20 StR_001 --> FR_001\n\
         ```\n"
    );
}

/// Trace: FR-040-AC-9
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_411_mermaid_cannot_be_broken_by_a_statements_punctuation() {
    // Each of these renders a broken diagram rather than a wrong one, which is
    // worse: nobody reviews a diagram that will not draw.
    let result = build_case(&input(
        bundle(),
        vec![obligation(
            "FR-001-AC-1",
            "Rejects (bad) input; logs \"why\"",
        )],
        vec![],
    ));
    let block = mermaid(&render(&result));

    assert!(block.contains("FR_001_AC_1"), "{block}");
    assert!(!block.contains("FR-001-AC-1("), "{block}");
    assert!(!block.contains(';'), "{block}");
    // `/"[^"\n]*"[^"\n]*"/` — a third quote on one line ends the label early.
    for line in block.lines() {
        assert!(
            line.matches('"').count() <= 2,
            "a label must carry no nested quote: {line}"
        );
    }

    // The literal the sanitiser produces, so the test says what the reader
    // sees rather than only what they will not see: `"` becomes `'` and `;`
    // becomes `,`, and the parentheses are left alone because the quotes are
    // what protects them.
    assert!(
        block.contains("  FR_001_AC_1([\"FR-001-AC-1: Rejects (bad) input, logs 'why'\"])\n"),
        "{block}"
    );
}

/// Trace: FR-040-AC-15
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#432
#[test]
fn tc_447_412_a_mermaid_label_truncates_on_a_code_point_while_node_ids_count_code_units() {
    // quoin#432. `String.prototype.slice` counts UTF-16 CODE UNITS, so a label
    // whose 80th unit fell inside a surrogate pair was cut in half and rendered
    // as U+FFFD.
    //
    // Constructed rather than taken from the corpus, and deliberately: no
    // label in the retained repository's 991 obligations reaches the boundary
    // with an astral character today, and waiting for a real one is waiting
    // for the bug.
    //
    // The label is `{id}: {statement}`, so the id and its separator are part
    // of the budget — the statement is padded to put the emoji astride the
    // 80-unit boundary of the whole label.
    let id = "FR-001-AC-1";
    let prefix = format!("{id}: ");
    let statement = format!("{}\u{1f600}", "x".repeat(80 - prefix.len()));
    let result = build_case(&input(bundle(), vec![obligation(id, &statement)], vec![]));
    let block = mermaid(&render(&result));

    // The emoji is dropped WHOLE rather than halved. In Rust a lone surrogate
    // cannot be held in a `String` at all, so the assertion that carries the
    // criterion is the truncation boundary itself, stated as a literal.
    assert!(
        block.contains(&format!(
            "([\"{prefix}{}\"])",
            "x".repeat(80 - prefix.len())
        )),
        "{block}"
    );
    assert!(!block.contains('\u{1f600}'), "{block}");
    assert!(!block.contains('\u{fffd}'), "{block}");
    // The whole block, not just the label: a replacement character anywhere in
    // the output is the defect, wherever it came from.
    assert!(std::str::from_utf8(block.as_bytes()).is_ok());

    // The other half, and the one a `chars()` port gets wrong in the opposite
    // direction: the node-id sanitiser emits one underscore per UTF-16 CODE
    // UNIT, so an astral character becomes TWO. Same defect family, read from
    // the retained regex's lack of a `u` flag rather than from preference.
    let astral_id = "FR-001-AC-\u{1f600}";
    let result = build_case(&input(
        bundle(),
        vec![obligation(astral_id, "It holds.")],
        vec![],
    ));
    let block = mermaid(&render(&result));
    // Three underscores after `AC`: one for the hyphen, TWO for the emoji.
    // A `chars()` port emits two and the node would still draw — which is why
    // the count is asserted rather than the diagram merely being checked.
    assert!(
        block.contains("  FR_001_AC___([\"FR-001-AC-\u{1f600}: It holds.\"])\n"),
        "{block}"
    );
    assert!(!block.contains("FR_001_AC__(["), "{block}");
}

/// Trace: FR-040-AC-15
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#432
#[test]
fn tc_447_413_a_label_with_no_astral_character_truncates_byte_for_byte_as_before() {
    // The fix must not move any byte that was already correct, which is 833 of
    // the retained repository's 835 truncated labels. A 100-character ASCII
    // statement exercises the truncation path; code points and code units
    // agree on every input where one of each is the other.
    let id = "FR-001-AC-1";
    let statement = "y".repeat(100);
    let result = build_case(&input(bundle(), vec![obligation(id, &statement)], vec![]));
    let block = mermaid(&render(&result));

    // No finding, so the node is supported and carries no ` ◇` suffix.
    let full = format!("{id}: {statement}");
    let expected: String = full.chars().take(80).collect();
    assert_eq!(expected.len(), 80);
    assert!(block.contains(&format!("([\"{expected}\"])")), "{block}");
}

/// Trace: FR-040-AC-10
/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[test]
fn tc_447_414_an_empty_case_says_there_is_no_case_rather_than_rendering_one() {
    // An empty document reads as "nothing to argue". The truth is that nothing
    // declared itself a claim, and those are different problems.
    let rendered = render(&build_case(&input(vec![], vec![], vec![])));

    assert!(rendered.contains("No document declares itself a top-level claim"));
    // The whole document: an empty case is short enough to state literally,
    // and "contains a sentence" would pass over a document that also rendered
    // a spurious claim count.
    assert_eq!(
        rendered,
        "# Assurance case\n\
         \n\
         No document declares itself a top-level claim, so there is no case to\n\
         argue. Declare an `StR` (or pass `--claim-type`) and re-run.\n"
    );
    assert!(!rendered.contains("claim(s)"));
}

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
