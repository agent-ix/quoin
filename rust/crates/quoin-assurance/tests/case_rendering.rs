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
use quoin_assurance::case::{CaseInput, NodeKind, NodeStatus, SuiteRef, Verdict, build_case};

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
#[trace("FR-040-AC-8", "TC-228")]
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
#[trace("FR-040-AC-9", "TC-229")]
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
#[trace("FR-040-AC-10", "TC-230")]
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
        "argued over `refines`: FR-003; `traces_to`: FR-001, FR-002"
    );
    assert_eq!(strategy.children.len(), 3);

    let rendered = render(&case);
    assert!(
        rendered.contains(
            "  - ◇ strategy — argued over `refines`: FR-003; `traces_to`: FR-001, FR-002\n"
        )
    );
    let graph = mermaid(&rendered);
    assert!(graph.contains(
        "  StR_001_strategy[/\"StR-001/strategy: argued over 'refines': FR-003, 'traces_to': FR-001, FR-002 ◇\"/]\n"
    ), "{graph}");
    assert!(graph.contains("  StR_001 --> StR_001_strategy\n"));
    assert!(graph.contains("  StR_001_strategy --> FR_001\n"));
}

/// A bound leaf lists its suites, the newest run or scan and the auditor's
/// verdict, in the markdown and the JSON, and re-rendering is byte-identical.
#[trace("FR-040-AC-17")]
#[test]
fn tc_363_002_a_bound_leaf_lists_suites_newest_run_and_the_auditors_verdict() {
    let request = |findings: serde_json::Value| -> CaseInput {
        let mut request = serde_json::json!({
            "documents": bundle(),
            "obligations": [
                {"id": "FR-001-AC-1", "statement": "It holds."},
                {"id": "FR-001-AC-2", "statement": "It also holds."}
            ],
            "findings": findings,
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
        request["unreadable"] = serde_json::json!([]);
        serde_json::from_value(request).expect("the request reads")
    };
    let findings = serde_json::json!([
        {"kind": "stale-evidence", "obligation": "FR-001-AC-2", "summary": "older than HEAD"}
    ]);

    let case = build_case(&request(findings.clone()));
    let leaves = &case.claims[0].children[0].children[0].children;
    let fresh = leaves[0].evidence.as_ref().expect("bound leaf");
    assert_eq!(fresh.verdict, Some(Verdict::Fresh));
    assert_eq!(fresh.newest.as_deref(), Some("2026-11-09T00:00:00Z"));
    assert_eq!(
        fresh.suites,
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
    let stale = leaves[1].evidence.as_ref().expect("bound leaf");
    assert_eq!(stale.verdict, Some(Verdict::Stale));
    assert_eq!(leaves[1].status, NodeStatus::Open);

    let rendered = render(&case);
    assert!(rendered.contains(
        "**FR-001-AC-1** — It holds.\n        - evidence: suites scan (2026-11-05T00:00:00Z), unit (2026-11-09T00:00:00Z); newest 2026-11-09T00:00:00Z; verdict fresh\n"
    ), "{rendered}");
    assert!(rendered.contains("verdict stale\n"));

    // JSON carries the same refs, and unchanged inputs give identical bytes.
    let json = serde_json::to_string(&case).expect("serialises");
    assert!(json.contains(r#""evidence":{"suites":[{"suite":"scan","at":"2026-11-05T00:00:00Z"}"#));
    assert!(json.contains(r#""verdict":"fresh""#));
    assert_eq!(
        json,
        serde_json::to_string(&build_case(&request(findings))).expect("serialises")
    );
}
