// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-040's rendering and mermaid criteria, against the public API (quoin#447).
//!
//! Byte-identical rendering, mermaid punctuation, label truncation on code
//! points while node ids count code units, and the empty case. The fixtures
//! are in `common/fixtures.rs`, which also carries the preamble this suite was
//! split out of.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_assurance::case::{CaseInput, NodeKind, NodeStatus, SuiteRef, build_case};
use quoin_assurance::matrix::EvidenceStatus;

#[path = "common/fixtures.rs"]
mod fixtures;
#[path = "common/render.rs"]
mod render_hop;

use crate::fixtures::{bundle, doc, input, obligation};
use crate::render_hop::render;

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

/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[trace("FR-040-AC-8")]
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
         \x20 - ✅ strategy — argued over `traces_to`: FR-001\n\
         \x20   - ✅ FR-001 — Parse the input\n\
         \x20     - ✅ **FR-001-AC-1** — It holds.\n\
         \n\
         ```mermaid\n\
         flowchart TD\n\
         \x20 StR_001[\"StR-001: The system is usable\"]\n\
         \x20 StR_001_strategy[/\"StR-001/strategy: argued over 'traces_to': FR-001\"/]\n\
         \x20 FR_001[\"FR-001: Parse the input\"]\n\
         \x20 FR_001_AC_1([\"FR-001-AC-1: It holds.\"])\n\
         \x20 FR_001 --> FR_001_AC_1\n\
         \x20 StR_001_strategy --> FR_001\n\
         \x20 StR_001 --> StR_001_strategy\n\
         ```\n"
    );
}

/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[trace("FR-040-AC-9")]
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

/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#432
#[trace("FR-040-AC-15")]
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

/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts, quoin#432
#[trace("FR-040-AC-15")]
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
    assert!(block.contains(&format!("([\"{expected}\"])")), "{block}");
}

/// Provenance: agent-ix/quoin#447, tests/assurance.test.ts
#[trace("FR-040-AC-10")]
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

/// A claim decomposed by two edge verbs gets one strategy node naming both,
/// in the markdown and in the mermaid graph.
#[trace("FR-040-AC-16")]
#[test]
fn tc_363_001_a_decomposition_renders_a_strategy_naming_its_edge_verbs() {
    let documents = vec![
        doc("StR-001", "StR", "Usable", &[]),
        doc("FR-002", "FR", "B", &[("StR-001", "traces_to")]),
        doc("FR-001", "FR", "A", &[("StR-001", "traces_to")]),
        doc("FR-003", "FR", "C", &[("StR-001", "refines")]),
    ];
    let case = build_case(&input(documents, vec![], vec![]));

    let strategy = &case.claims[0].children[0];
    assert_eq!(strategy.kind, NodeKind::Strategy);
    assert_eq!(strategy.id, "StR-001/strategy");
    assert_eq!(
        strategy.statement,
        "argued over `refines`: FR-003 | `traces_to`: FR-001, FR-002"
    );
    assert_eq!(strategy.children.len(), 3);

    let rendered = render(&case);
    assert!(rendered.contains(
        "  - ◇ strategy — argued over `refines`: FR-003 | `traces_to`: FR-001, FR-002\n"
    ));
    // The verb groups stay distinguishable after sanitising: `|` is not a
    // character the label rewrites, and ids never contain it.
    let graph = mermaid(&rendered);
    assert!(
        graph.contains(
            "  StR_001_strategy[/\"StR-001/strategy: argued over 'refines': FR-003 | 'traces_to': FR-001, FR-002 ◇\"/]\n"
        ),
        "{graph}"
    );
    assert!(graph.contains("  StR_001 --> StR_001_strategy\n"));
    assert!(graph.contains("  StR_001_strategy --> FR_001\n"));
}

/// A child declaring two verbs toward one parent is one sub-claim, argued
/// over both verbs.
#[trace("FR-040-AC-16")]
#[test]
fn tc_363_003_a_child_with_two_verbs_toward_one_parent_appears_once_under_both() {
    let documents = vec![
        doc("StR-001", "StR", "Usable", &[]),
        doc(
            "FR-001",
            "FR",
            "A",
            &[("StR-001", "traces_to"), ("StR-001", "refines")],
        ),
    ];
    let case = build_case(&input(documents, vec![], vec![]));
    let strategy = &case.claims[0].children[0];
    assert_eq!(
        strategy.statement,
        "argued over `refines`: FR-001 | `traces_to`: FR-001"
    );
    assert_eq!(strategy.children.len(), 1);
}

/// One bound-leaf request over the bundle: `AC-1` bound to `unit` and `scan`,
/// `AC-2` to `unit`; extra members come from `extra`.
fn leaf_request(extra: &serde_json::Value) -> CaseInput {
    let mut request = serde_json::json!({
        "documents": bundle(),
        "obligations": [
            {"id": "FR-001-AC-1", "statement": "It holds."},
            {"id": "FR-001-AC-2", "statement": "It also holds."}
        ],
        "findings": [],
        "bindings": [
            {"obligation": "FR-001-AC-1", "suite": "unit", "symbols": []},
            {"obligation": "FR-001-AC-1", "suite": "scan", "symbols": []},
            {"obligation": "FR-001-AC-2", "suite": "unit", "symbols": []}
        ],
        "observations": [
            {"suite": "unit", "timestamp": "2026-11-01T00:00:00Z"},
            {"suite": "unit", "timestamp": "2026-11-09T00:00:00Z"},
            {"suite": "scan", "timestamp": "2026-11-05T00:00:00Z"}
        ],
    });
    for (key, value) in extra.as_object().expect("extra is an object") {
        request[key] = value.clone();
    }
    serde_json::from_value(request).expect("the request reads")
}

fn leaves(case: &quoin_assurance::case::AssuranceCase) -> &[quoin_assurance::case::CaseNode] {
    &case.claims[0].children[0].children[0].children
}

/// A bound leaf lists its suites, the newest run or scan and the auditor's
/// verdict, in the markdown and the JSON, byte-identically over unchanged input.
#[trace("FR-040-AC-17")]
#[test]
fn tc_363_002_a_bound_leaf_lists_suites_newest_run_and_the_auditors_verdict() {
    let extra = serde_json::json!({
        "healthy": ["FR-001-AC-1"],
        "findings": [{"kind": "stale-evidence", "obligation": "FR-001-AC-2", "summary": "older than HEAD"}],
    });
    let case = build_case(&leaf_request(&extra));
    let leaves = leaves(&case);
    let bound = leaves[0].evidence.as_ref().expect("bound leaf");
    assert_eq!(bound.verdict, Some(EvidenceStatus::Bound));
    assert_eq!(bound.newest.as_deref(), Some("2026-11-09T00:00:00Z"));
    assert_eq!(
        bound.suites,
        vec![
            SuiteRef {
                suite: "scan".to_owned(),
                at: Some("2026-11-05T00:00:00Z".to_owned())
            },
            SuiteRef {
                suite: "unit".to_owned(),
                at: Some("2026-11-09T00:00:00Z".to_owned())
            },
        ]
    );
    assert_eq!(leaves[0].status, NodeStatus::Supported);
    let stale = leaves[1].evidence.as_ref().expect("bound leaf");
    assert_eq!(stale.verdict, Some(EvidenceStatus::Stale));
    assert_eq!(leaves[1].status, NodeStatus::Open);

    let rendered = render(&case);
    assert!(rendered.contains(
        "**FR-001-AC-1** — It holds.\n        - evidence: suites scan (2026-11-05T00:00:00Z), unit (2026-11-09T00:00:00Z); newest 2026-11-09T00:00:00Z; verdict bound\n"
    ), "{rendered}");
    assert!(rendered.contains("verdict stale\n"));

    let json = serde_json::to_string(&case).expect("serialises");
    assert!(json.contains(r#""evidence":{"suites":[{"suite":"scan","at":"2026-11-05T00:00:00Z"}"#));
    assert!(json.contains(r#""verdict":"bound""#));
    let again = serde_json::to_string(&build_case(&leaf_request(&extra))).expect("serialises");
    assert_eq!(json, again);
}

/// `bound` is the auditor's `healthy`, not "no finding": a ladder that stopped
/// early raises nothing and is still not healthy, so the leaf is open and says
/// so; an unevaluated check keeps its reason on the leaf.
#[trace("FR-040-AC-18")]
#[test]
fn tc_363_004_a_leaf_the_audit_does_not_list_healthy_is_open_not_bound() {
    let case = build_case(&leaf_request(&serde_json::json!({
        "healthy": [],
        "unevaluated": [{"check": "mocked-confirmation", "obligation": "FR-001-AC-2",
                         "suites": ["unit"], "reason": "no current mock inspection exists for unit."}],
    })));
    let leaves = leaves(&case);

    assert_eq!(leaves[0].status, NodeStatus::Open);
    assert_eq!(
        leaves[0].because.as_deref(),
        Some("not healthy: the audit reached no verdict for this obligation")
    );
    assert_eq!(
        leaves[0].evidence.as_ref().expect("bound").verdict,
        Some(EvidenceStatus::Undischarged)
    );
    assert_eq!(leaves[1].status, NodeStatus::Open);
    assert_eq!(
        leaves[1].because.as_deref(),
        Some("unevaluated mocked-confirmation: no current mock inspection exists for unit.")
    );
    assert!(render(&case).contains("verdict undischarged"));
}

/// Suspect outranks stale, as in the matrix; another finding kind is
/// `undischarged`; a bound suite with no run or scan says so.
#[trace("FR-040-AC-17")]
#[test]
fn tc_363_005_verdicts_follow_the_matrix_precedence_and_missing_runs_are_visible() {
    let case = build_case(&leaf_request(&serde_json::json!({
        "healthy": [],
        "findings": [
            {"kind": "stale-evidence", "obligation": "FR-001-AC-1", "summary": "old"},
            {"kind": "mocked-confirmation", "obligation": "FR-001-AC-1", "summary": "mocked"},
            {"kind": "method-conformance", "obligation": "FR-001-AC-2", "summary": "wrong kind"}
        ],
        "observations": [{"suite": "scan", "timestamp": "2026-11-05T00:00:00Z"}],
    })));
    let leaves = leaves(&case);
    let first = leaves[0].evidence.as_ref().expect("bound");
    assert_eq!(first.verdict, Some(EvidenceStatus::Suspect));
    assert_eq!(
        leaves[1].evidence.as_ref().expect("bound").verdict,
        Some(EvidenceStatus::Undischarged)
    );
    let unit = leaves[1].evidence.as_ref().expect("bound");
    assert_eq!(
        unit.suites,
        vec![SuiteRef {
            suite: "unit".to_owned(),
            at: None
        }]
    );
    assert_eq!(unit.newest, None);
    assert!(render(&case).contains("unit (no run or scan recorded); newest none recorded"));
}

/// "Newest" is the later instant, not the later string: fractional seconds and
/// zone offsets order lexically wrong.
#[trace("FR-040-AC-19")]
#[test]
fn tc_363_006_newest_compares_instants_not_strings() {
    let case = build_case(&leaf_request(&serde_json::json!({
        "observations": [
            // Lexically `…00Z` > `…00.500Z`; the instant is half a second later.
            {"suite": "unit", "timestamp": "2026-11-09T00:00:00Z"},
            {"suite": "unit", "timestamp": "2026-11-09T00:00:00.500Z"},
            // 12:00+05:00 is 07:00Z, earlier than 10:00Z, though it sorts later.
            {"suite": "scan", "timestamp": "2026-11-09T10:00:00Z"},
            {"suite": "scan", "timestamp": "2026-11-09T12:00:00+05:00"}
        ],
    })));
    let evidence = leaves(&case)[0].evidence.as_ref().expect("bound");
    let at = |suite: &str| {
        evidence
            .suites
            .iter()
            .find(|s| s.suite == suite)
            .and_then(|s| s.at.as_deref())
    };
    assert_eq!(at("unit"), Some("2026-11-09T00:00:00.500Z"));
    assert_eq!(at("scan"), Some("2026-11-09T10:00:00Z"));
    assert_eq!(evidence.newest.as_deref(), Some("2026-11-09T10:00:00Z"));
}

/// Analysis results attach to the full native target and remain an additive
/// display field; they do not alter the legacy audit status.
#[trace("FR-040-AC-20", "FR-153-AC-12")]
#[test]
fn analysis_solution_shows_exact_target_results_in_json_and_text() {
    let target = serde_json::json!({
        "source": "requirements",
        "document": "spec/FR-001.md",
        "id": "FR-001-AC-1",
        "origin": "authored",
        "row": 1,
        "line": 12,
    });
    let wrong_target = serde_json::json!({
        "source": "requirements",
        "document": "spec/FR-002.md",
        "id": "FR-001-AC-1",
        "origin": "authored",
        "row": 1,
        "line": 12,
    });
    let mut input = leaf_request(&serde_json::json!({ "healthy": [] }));
    input.obligations[0].method = Some("Analysis".to_owned());
    input.obligations[0].identity = Some(serde_json::from_value(target.clone()).expect("target"));
    input.obligations[1].identity = Some(
        serde_json::from_value(serde_json::json!({
            "source": "requirements",
            "document": "spec/FR-001.md",
            "id": "FR-001-AC-2",
            "origin": "authored",
            "row": 2,
            "line": 13,
        }))
        .expect("other target"),
    );
    input.analysis_evidence = Some(
        serde_json::from_value(serde_json::json!({
            "record_binders": [{
                "target": target,
                "locator": {"record": "runs/unit/2026-11-13.json", "run": "r1", "entry": "e1"},
                "method": "Analysis",
                "actor": "ci",
                "provenance": ["signed-host"],
            }],
            "evidence_results": [{
                "target": wrong_target,
                "requested_target": "FR-001-AC-1",
                "cause": "stale",
                "detail": "belongs to a different document",
            }],
        }))
        .expect("analysis evidence"),
    );
    let case = build_case(&input);
    let leaves = leaves(&case);
    assert_eq!(
        leaves[0]
            .analysis_evidence
            .as_ref()
            .expect("Analysis field")
            .len(),
        1
    );
    assert_eq!(leaves[0].status, NodeStatus::Open);
    assert!(leaves[1].analysis_evidence.is_none());

    let json = serde_json::to_value(&case).expect("serialises case");
    assert_eq!(
        json["claims"][0]["children"][0]["children"][0]["children"][0]["analysis_evidence"][0]["status"],
        "qualified-record"
    );
    let markdown = render(&case);
    assert!(
        markdown.contains("analysis evidence: {\"record_binder\":"),
        "{markdown}"
    );
    assert!(!markdown.contains("belongs to a different document"));
}
