// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The FR-062 criteria `tests/graph-analysis.test.ts` carried, restated here.
//!
//! # Why this file exists
//!
//! quoin#500 deleted `src/graph-analysis/` and the 630-line vitest file that
//! was the only home of FR-062-AC-4 through AC-12, FR-062-CON-2 and
//! FR-062-CON-3. `tc_385_parity.rs` already pins what the three views COMPUTE,
//! against the captured oracle corpus, but it does so under three coarse tags
//! (AC-1, AC-2, AC-3): a criterion that is exercised by a corpus rung nobody
//! named is a criterion the matrix reads as unbacked. So each deleted criterion
//! is restated here as its own named assertion over its own fixture, in the
//! same shape the vitest case asserted it.
//!
//! These are **not** re-derivations of the implementation. Every expectation is
//! written out as a literal — the row order, the depth, the deduplicated event,
//! the gap kind — exactly as the TypeScript wrote it, so a test that agrees
//! with itself no matter what the code does is not what replaced the ones that
//! were deleted.
//!
//! # The fixture is the deleted file's fixture
//!
//! Four requirements, one acceptance criterion each, one module, one accepted
//! source. It is built as JSON and read through this crate's own parsers rather
//! than by constructing the model types, because that is the path every caller
//! takes and a hand-built value could satisfy a shape no document can reach.
//!
//! Trace: FR-062-AC-4, FR-062-AC-5, FR-062-AC-6, FR-062-AC-7, FR-062-AC-8,
//! FR-062-AC-9, FR-062-AC-10, FR-062-AC-11, FR-062-AC-12, FR-062-CON-2,
//! FR-062-CON-3
//!
//! Provenance: quoin#500

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_graph_analysis::model::binding::{BindingInput, parse_bindings_file};
use quoin_graph_analysis::{
    ArtifactId, DEFAULT_RELATION_KINDS, GraphAnalysis, GraphAnalysisInput, RelationKind,
    analyze_change_impact, analyze_churn, analyze_fan_out, check_accepted_premises,
    check_audit_identity, parse_accepted_premises, parse_assurance_export, parse_audit_envelope,
    render_graph_analysis, render_graph_analysis_json,
};
use serde_json::{Value, json};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_DIGEST: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const SCHEMA_DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OTHER_REVISION: &str = "dddddddddddddddddddddddddddddddddddddddd";

const REQUIREMENTS: [&str; 4] = ["FR-001", "FR-002", "FR-003", "FR-004"];

fn locator(path: &str, line: u32) -> Value {
    json!({ "path": path, "line": line, "digest": DIGEST })
}

fn source() -> Value {
    json!({ "repository": "agent-ix/example", "revision": REVISION })
}

fn modules() -> Value {
    json!([{
        "name": "example",
        "version": "1.0.0",
        "schemas": [{ "archetype": "FR", "schema_digest": SCHEMA_DIGEST }],
    }])
}

fn premises_value() -> Value {
    json!({ "format": "quire-assurance", "format_version": 1, "modules": modules() })
}

fn artifact(id: &str, artifact_type: &str) -> Value {
    json!({
        "id": id,
        "artifact_type": artifact_type,
        "locator": locator(&format!("spec/{id}.md"), 1),
    })
}

fn obligation(id: &str, owner: &str) -> Value {
    json!({
        "source": "acceptance-criterion",
        "id": id,
        "document": format!("spec/{owner}.md"),
        "statement": format!("{id} statement"),
        "statement_hash": DIGEST,
        "target_ids": [format!("TC-{id}")],
        "locator": locator(&format!("spec/{owner}.md"), 20),
    })
}

fn relation(from: &str, to: &str, edge_type: &str) -> Value {
    json!({
        "kind": "corpus",
        "source": from,
        "target": to,
        "edge_type": edge_type,
        "resolution": "resolved",
        "locator": locator(&format!("spec/{from}.md"), 5),
        "freshness": "not_applicable",
    })
}

fn relation_kinds() -> Value {
    DEFAULT_RELATION_KINDS
        .iter()
        .map(|kind| {
            json!({
                "kind": kind,
                "availability": "available",
                "sources": ["module_vocabulary"],
            })
        })
        .collect::<Vec<_>>()
        .into()
}

fn artifacts() -> Vec<Value> {
    REQUIREMENTS.iter().map(|id| artifact(id, "FR")).collect()
}

fn obligations() -> Vec<Value> {
    REQUIREMENTS
        .iter()
        .map(|id| obligation(&format!("{id}-AC-1"), id))
        .collect()
}

/// The four-requirement export the deleted vitest fixture built.
fn export_value() -> Value {
    json!({
        "format": "quire-assurance",
        "format_version": 1,
        "modules": modules(),
        "source": source(),
        "artifacts": artifacts(),
        "obligations": obligations(),
        "symbols": [],
        "relation_kinds": relation_kinds(),
        "relations": [],
        "relation_observations": [],
    })
}

/// The four-node cycle the change-impact criteria are stated over.
fn graph_relations() -> Vec<Value> {
    vec![
        relation("FR-002", "FR-001", "depends_on"),
        relation("FR-003", "FR-001", "derives_from"),
        relation("FR-004", "FR-002", "requires"),
        relation("FR-004", "FR-003", "refines"),
        relation("FR-001", "FR-004", "depends_on"),
    ]
}

fn selected() -> Vec<RelationKind> {
    ["depends_on", "derives_from", "requires", "refines"]
        .iter()
        .map(|kind| RelationKind::new(*kind))
        .collect()
}

fn audit_value(healthy: &[&str], findings: &Value) -> Value {
    json!({
        "format": "quoin-audit-envelope",
        "format_version": 1,
        "source": source(),
        "export": premises_value(),
        "report": { "findings": findings, "healthy": healthy, "unevaluated": [] },
    })
}

fn binding(obligation: &str, suite: &str, affirmations: Option<Value>) -> Value {
    let mut value = json!({
        "obligation": obligation,
        "statementHashAtBinding": DIGEST,
        "suite": suite,
        "commit": REVISION,
        "symbols": ["test"],
    });
    if let Some(affirmations) = affirmations {
        value["affirmations"] = affirmations;
    }
    value
}

fn available(bindings: &[Value]) -> BindingInput {
    BindingInput::Available(
        parse_bindings_file(&json!({
            "schemaVersion": quoin_store::STORE_SCHEMA_VERSION,
            "bindings": bindings,
        }))
        .expect("the fixture bindings satisfy the retained schema"),
    )
}

/// One analysis input, read through this crate's own contracts.
fn input(export: &Value, bindings: BindingInput, audit: &Value) -> GraphAnalysisInput {
    GraphAnalysisInput {
        assurance: parse_assurance_export(&export.to_string()).expect("the fixture export parses"),
        premises: parse_accepted_premises(&premises_value().to_string())
            .expect("the fixture premises parse"),
        audit: parse_audit_envelope(&audit.to_string()).expect("the fixture audit parses"),
        bindings,
    }
}

/// The plain four-requirement input with the bindings a case names.
fn plain(bindings: &[Value]) -> GraphAnalysisInput {
    input(
        &export_value(),
        available(bindings),
        &audit_value(
            &["FR-001-AC-1", "FR-002-AC-1", "FR-003-AC-1", "FR-004-AC-1"],
            &json!([]),
        ),
    )
}

/// One report as a JSON value, through the renderer a caller actually reaches.
fn as_json(analysis: &GraphAnalysis) -> Value {
    serde_json::from_str(
        &render_graph_analysis_json(analysis).expect("the report has a canonical spelling"),
    )
    .expect("the canonical rendering is JSON")
}

/// Every reached requirement joins its obligations and suites; an unknown or
/// non-requirement seed is a named gap with no partial closure for that seed.
///
/// Restates `tests/graph-analysis.test.ts` "joins every reached requirement and
/// isolates an unknown seed".
///
/// Trace: FR-062-AC-4
/// Provenance: quoin#500
#[test]
fn tc_500_040_a_reached_requirement_joins_and_an_unknown_seed_is_isolated() {
    let mut export = export_value();
    export["artifacts"]
        .as_array_mut()
        .unwrap()
        .push(artifact("PLAN-999", "Plan"));
    export["obligations"]
        .as_array_mut()
        .unwrap()
        .push(obligation("PLAN-999-AC-1", "PLAN-999"));
    let mut relations = graph_relations();
    relations.push(relation("PLAN-999", "FR-001", "depends_on"));
    export["relations"] = Value::from(relations);

    let bindings = vec![
        binding("FR-001-AC-1", "unit", None),
        binding("FR-002-AC-1", "integration", None),
        binding("FR-004-AC-1", "integration", None),
        binding("PLAN-999-AC-1", "planning", None),
    ];
    let analysis = analyze_change_impact(
        &input(
            &export,
            available(&bindings),
            &audit_value(
                &["FR-001-AC-1", "FR-002-AC-1", "FR-003-AC-1", "FR-004-AC-1"],
                &json!([]),
            ),
        ),
        &["FR-404", "PLAN-999", "FR-001"].map(ArtifactId::new),
        Some(&selected()),
    )
    .expect("the impact report is computable");
    let report = as_json(&GraphAnalysis::from(analysis));

    let requirements: Vec<&str> = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["requirement"].as_str().expect("a requirement id"))
        .collect();
    assert!(
        !requirements.contains(&"FR-404"),
        "an unknown seed must reach no row of its own"
    );
    assert!(
        !requirements.contains(&"PLAN-999"),
        "a non-requirement seed must reach no row of its own, even when the corpus resolves it"
    );

    let reached = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["requirement"] == "FR-002")
        .expect("FR-002 is reached from FR-001");
    assert_eq!(reached["obligations"][0]["obligation"], "FR-002-AC-1");
    assert_eq!(
        reached["obligations"][0]["requirements"],
        json!(["FR-002"]),
        "a reached obligation names every requirement that owns it"
    );
    assert_eq!(
        reached["obligations"][0]["bindings"][0]["suite"],
        "integration"
    );

    for subject in ["FR-404", "PLAN-999"] {
        assert!(
            report["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|gap| { gap["kind"] == "unknown-requirement" && gap["subject"] == subject }),
            "{subject} must be reported as an unknown-requirement gap"
        );
    }
}

/// A reachable supported binding stays supported while change exposure is
/// reported separately, and every other auditor verdict is copied byte for byte.
///
/// Restates "copies the existing auditor verdict apart from exposure".
///
/// Trace: FR-062-AC-5, FR-062-CON-3
/// Provenance: quoin#500
#[test]
fn tc_500_041_the_auditor_verdict_is_copied_and_never_re_decided() {
    let finding = json!({
        "kind": "stale-evidence",
        "obligation": "FR-002-AC-1",
        "severity": "medium",
        "summary": "The retained run is behind the accepted revision.",
    });
    let mut export = export_value();
    export["relations"] = Value::from(graph_relations());
    let analysis = analyze_change_impact(
        &input(
            &export,
            available(&[
                binding("FR-001-AC-1", "unit", None),
                binding("FR-002-AC-1", "integration", None),
                binding("FR-004-AC-1", "integration", None),
            ]),
            &audit_value(
                &["FR-001-AC-1", "FR-003-AC-1", "FR-004-AC-1"],
                &json!([finding.clone()]),
            ),
        ),
        &[ArtifactId::new("FR-001")],
        Some(&selected()),
    )
    .expect("the impact report is computable");
    let report = as_json(&GraphAnalysis::from(analysis));

    let verdict = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["requirement"] == "FR-002")
        .expect("FR-002 is reached")["obligations"][0]["bindings"][0]["auditorVerdict"]
        .clone();
    assert_eq!(
        verdict,
        json!({ "findings": [finding], "healthy": [], "unevaluated": [] }),
        "the auditor's own verdict is copied, not recomputed"
    );

    let text = report.to_string();
    assert!(
        text.contains("stale-evidence"),
        "the existing finding must survive into the view"
    );
    assert!(
        !text.contains("suspectObligations"),
        "change exposure must be reported separately and must not re-grade a binding"
    );
}

/// One affirmation copied across several suite bindings deduplicates by
/// obligation, actor, commit and note, while retaining every affected suite —
/// whichever order the store lists the bindings in.
///
/// Restates "deduplicates one affirmation while retaining all affected suites".
///
/// Trace: FR-062-AC-6
/// Provenance: quoin#500
#[test]
fn tc_500_042_one_affirmation_deduplicates_and_keeps_every_suite() {
    let event = json!({ "who": "@reviewer", "commit": REVISION, "note": "still valid" });
    for reverse in [false, true] {
        let mut bindings = vec![
            binding("FR-001-AC-1", "unit", Some(json!([event]))),
            binding("FR-001-AC-1", "integration", Some(json!([event]))),
        ];
        if reverse {
            bindings.reverse();
        }
        let report = as_json(&GraphAnalysis::from(analyze_churn(&plain(&bindings))));
        let row = &report["rows"][0];
        assert_eq!(row["obligation"], "FR-001-AC-1");
        assert_eq!(
            row["eventCount"], 1,
            "reverse={reverse}: one event, not two"
        );
        assert_eq!(
            row["events"][0],
            json!({
                "who": "@reviewer",
                "commit": REVISION,
                "note": "still valid",
                "suites": ["integration", "unit"],
            }),
            "reverse={reverse}: the one event names every suite it was copied across"
        );
    }
}

/// Churn retains zero-event live obligations, sorts count-descending then id,
/// and reports affirmation history for an absent obligation only as a gap.
///
/// Restates "retains zero-event rows and gaps orphan affirmation history".
///
/// Trace: FR-062-AC-7
/// Provenance: quoin#500
#[test]
fn tc_500_043_churn_retains_zero_event_rows_and_gaps_an_orphan_history() {
    let event = json!({ "who": "@reviewer", "commit": REVISION, "note": "still valid" });
    let report = as_json(&GraphAnalysis::from(analyze_churn(&plain(&[
        binding("FR-002-AC-1", "unit", Some(json!([event]))),
        binding("FR-999-AC-1", "old", Some(json!([event]))),
    ]))));

    let rows: Vec<(&str, u64)> = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["obligation"].as_str().expect("an obligation id"),
                row["eventCount"].as_u64().expect("a count"),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("FR-002-AC-1", 1),
            ("FR-001-AC-1", 0),
            ("FR-003-AC-1", 0),
            ("FR-004-AC-1", 0),
        ],
        "count-descending, then id; a live obligation with no history is a zero row, not an \
         absent one"
    );
    assert!(
        report["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["subject"] == "old:FR-999-AC-1"),
        "history for an obligation the export does not carry is a gap, never a row"
    );
}

/// Every view carries the accepted full source revision and the format/module
/// premises, byte for byte.
///
/// Restates "preserves source and accepted premises across every view".
///
/// Trace: FR-062-AC-8
/// Provenance: quoin#500
#[test]
fn tc_500_044_every_view_carries_the_accepted_source_and_premises() {
    let value = plain(&[binding("FR-001-AC-1", "unit", None)]);
    let views = [
        GraphAnalysis::from(analyze_fan_out(&value)),
        GraphAnalysis::from(analyze_churn(&value)),
        GraphAnalysis::from(
            analyze_change_impact(&value, &[ArtifactId::new("FR-001")], None)
                .expect("the impact report is computable"),
        ),
    ];
    for view in &views {
        let report = as_json(view);
        assert_eq!(
            report["source"],
            source(),
            "the full revision, never a prefix"
        );
        assert_eq!(report["premises"], premises_value());
        assert_eq!(
            report["export"],
            json!({ "format": "quire-assurance", "format_version": 1 })
        );
    }
}

/// Invalid inputs, non-exact premises and a mismatched audit identity fail
/// closed; an unavailable retained store never appears as a healthy zero, and
/// absent is not the same answer as unreadable.
///
/// Restates "rejects invalid premises/audit identity and distinguishes
/// unavailable bindings".
///
/// Trace: FR-062-AC-9
/// Provenance: quoin#500
#[test]
fn tc_500_045_bad_inputs_fail_closed_and_unavailable_is_not_zero() {
    assert!(parse_accepted_premises("not json").is_err());
    assert!(parse_audit_envelope("{}").is_err());
    assert!(parse_assurance_export("{}").is_err());

    let export = parse_assurance_export(&export_value().to_string()).expect("the export parses");
    let with_modules = |modules: Value| {
        let mut value = premises_value();
        value["modules"] = modules;
        parse_accepted_premises(&value.to_string()).expect("the candidate premises parse")
    };

    // Fewer modules, more schemas, and a module the export does not carry: each
    // is a premise set that is not EXACTLY the accepted one.
    assert!(check_accepted_premises(&export, &with_modules(json!([]))).is_err());
    let mut widened = modules();
    widened[0]["schemas"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "archetype": "US", "schema_digest": DIGEST }));
    assert!(check_accepted_premises(&export, &with_modules(widened)).is_err());
    let mut extra = modules();
    extra.as_array_mut().unwrap().push(json!({
        "name": "not-in-export",
        "version": "1.0.0",
        "schemas": [{ "archetype": "FR", "schema_digest": DIGEST }],
    }));
    assert!(check_accepted_premises(&export, &with_modules(extra)).is_err());

    let mut moved = audit_value(&["FR-001-AC-1"], &json!([]));
    moved["source"]["revision"] = json!(OTHER_REVISION);
    let moved = parse_audit_envelope(&moved.to_string()).expect("the candidate envelope parses");
    assert!(
        check_audit_identity(&moved, &export).is_err(),
        "an audit taken at another revision is not this export's audit"
    );

    let with_store = |bindings: BindingInput| {
        as_json(&GraphAnalysis::from(analyze_fan_out(&input(
            &export_value(),
            bindings,
            &audit_value(&["FR-001-AC-1"], &json!([])),
        ))))
    };
    let absent = with_store(BindingInput::Absent {
        reason: "missing".to_owned(),
    });
    let unreadable = with_store(BindingInput::Unreadable {
        reason: "bad JSON".to_owned(),
    });
    let empty = with_store(available(&[]));

    for (name, report) in [("absent", &absent), ("unreadable", &unreadable)] {
        assert_eq!(report["state"], "not_computed", "{name}");
        assert_eq!(report["rows"], json!([]), "{name}");
    }
    assert_ne!(
        absent["gaps"][0]["kind"], unreadable["gaps"][0]["kind"],
        "a repository that recorded nothing and one whose evidence cannot be read are not the \
         same answer"
    );
    assert_eq!(
        empty["state"], "incomplete",
        "a readable but empty store is incomplete, not not_computed"
    );
    assert!(
        empty["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["kind"] == "empty-bindings-store")
    );

    // A relationship the export does not offer is not a walk with fewer edges.
    let mut narrowed = export_value();
    narrowed["relation_kinds"] = narrowed["relation_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|kind| kind["kind"] != "depends_on")
        .cloned()
        .collect::<Vec<_>>()
        .into();
    let unsupported = analyze_change_impact(
        &input(
            &narrowed,
            available(&[]),
            &audit_value(&["FR-001-AC-1"], &json!([])),
        ),
        &[ArtifactId::new("FR-001")],
        None,
    )
    .expect("the impact report is computable");
    let unsupported = as_json(&GraphAnalysis::from(unsupported));
    assert_eq!(unsupported["state"], "not_computed");
    assert_eq!(unsupported["rows"], json!([]));
}

/// Reordered equivalent premises, auditor verdict arrays and graph inputs emit
/// byte-identical JSON, and the human rendering comes from the same report.
///
/// Restates "renders equivalent permutations as identical canonical JSON".
///
/// Trace: FR-062-AC-10
/// Provenance: quoin#500
#[test]
fn tc_500_046_equivalent_permutations_render_identical_bytes() {
    let bindings = vec![
        binding("FR-002-AC-1", "z", None),
        binding("FR-001-AC-1", "a", None),
    ];
    let left = analyze_fan_out(&plain(&bindings));

    let mut reversed_export = export_value();
    reversed_export["artifacts"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reversed_export["obligations"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let mut reversed_bindings = bindings.clone();
    reversed_bindings.reverse();
    let right = analyze_fan_out(&input(
        &reversed_export,
        available(&reversed_bindings),
        &audit_value(
            &["FR-001-AC-1", "FR-002-AC-1", "FR-003-AC-1", "FR-004-AC-1"],
            &json!([]),
        ),
    ));

    let left = GraphAnalysis::from(left);
    assert_eq!(
        render_graph_analysis_json(&left).unwrap(),
        render_graph_analysis_json(&GraphAnalysis::from(right)).unwrap(),
        "two spellings of one input must render the same bytes"
    );

    let markdown = render_graph_analysis(&left);
    assert!(
        markdown.contains(REVISION),
        "the human rendering carries the full accepted revision"
    );
    assert!(markdown.contains("Suite"));
}

/// The two input canonicalizers agree across a permutation: a reordered module
/// list is one premise set, and a reordered audit report is one audit.
///
/// The other half of "renders equivalent permutations as identical canonical
/// JSON" — the report is only canonical if what it repeats back was made
/// canonical on the way in, which is a fact about the contracts and not about
/// the renderer.
///
/// Trace: FR-062-AC-10
/// Provenance: quoin#500
#[test]
fn tc_500_049_a_permuted_premise_set_and_audit_parse_to_one_value() {
    let two_modules = |reverse: bool| {
        let mut list = modules().as_array().unwrap().clone();
        list.push(json!({
            "name": "second",
            "version": "2.0.0",
            "schemas": [
                { "archetype": "US", "schema_digest": OTHER_DIGEST },
                { "archetype": "NFR", "schema_digest": "e".repeat(64) },
            ],
        }));
        if reverse {
            list.reverse();
            for module in &mut list {
                module["schemas"].as_array_mut().unwrap().reverse();
            }
        }
        let mut value = premises_value();
        value["modules"] = Value::from(list);
        parse_accepted_premises(&value.to_string()).expect("the premises parse")
    };
    assert_eq!(
        two_modules(false),
        two_modules(true),
        "module and schema order is not a fact about a premise set"
    );

    let findings = [
        json!({
            "kind": "stale-evidence",
            "obligation": "FR-001-AC-1",
            "severity": "medium",
            "summary": "stale",
        }),
        json!({
            "kind": "unknown-method",
            "obligation": "FR-001-AC-1",
            "severity": "low",
            "summary": "unknown",
        }),
    ];
    let unevaluated = |suites: Value, reason: &str| {
        json!({
            "check": "mocked-confirmation",
            "obligation": "FR-001-AC-1",
            "suites": suites,
            "reason": reason,
        })
    };
    let envelope = |reverse: bool| {
        let mut list = findings.to_vec();
        // One suite list is itself permuted, so this covers both the order of
        // the checks and the order inside one of them.
        let mut checks = vec![
            unevaluated(json!(["unit", "integration"]), "not inspected"),
            unevaluated(json!(["system"]), "not recorded"),
        ];
        if reverse {
            list.reverse();
            checks = vec![
                unevaluated(json!(["system"]), "not recorded"),
                unevaluated(json!(["integration", "unit"]), "not inspected"),
            ];
        }
        let mut value = audit_value(&["FR-001-AC-1", "FR-001-AC-1"], &Value::from(list));
        value["report"]["unevaluated"] = Value::from(checks);
        parse_audit_envelope(&value.to_string()).expect("the envelope parses")
    };
    assert_eq!(
        envelope(false),
        envelope(true),
        "finding, healthy and unevaluated order is not a fact about an audit"
    );
}

/// The analysis half reaches no producer, no subprocess, no network and no
/// write, and the ONE filesystem seam is the reader in `src/load.rs`.
///
/// Restates the `src/graph-analysis/` half of
/// `tests/graph-command.test.ts`'s "keeps producers, writes, frontmatter, and a
/// second graph outside every view". The COMMAND half of that criterion stays
/// on the TypeScript side, where the commands still live.
///
/// Trace: FR-062-AC-11, FR-062-CON-1, FR-062-CON-4
/// Provenance: quoin#500
#[test]
fn tc_500_047_the_analysis_half_reaches_nothing_but_its_one_reader() {
    /// The one module permitted to name the filesystem, and why.
    const READER: &str = "load.rs";
    /// Below this the census is reading nothing.
    const SOURCE_FLOOR: usize = 10;
    /// Capabilities no projection may reach for.
    const FORBIDDEN: &[&str] = &[
        "std::process",
        "std::net",
        "std::env",
        "Command::new",
        "reqwest",
        "write_to_string",
        "File::create",
    ];

    fn walk(directory: &Path, into: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(directory).expect("src/ is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .file_name()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                into.push((name, std::fs::read_to_string(&path).expect("a source file")));
            }
        }
    }

    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    walk(&root, &mut sources);
    assert!(
        sources.len() >= SOURCE_FLOOR,
        "anti-vacuity floor: at least {SOURCE_FLOOR} modules expected, saw {} — a census that \
         finds nothing proves nothing",
        sources.len()
    );

    let mut readers = 0_usize;
    for (name, text) in &sources {
        for needle in FORBIDDEN {
            assert!(
                !text.contains(needle),
                "{name} names {needle}; a read-only projection runs no producer, opens no socket \
                 and writes nothing"
            );
        }
        if text.contains("std::fs") {
            assert_eq!(
                name, READER,
                "{name} names std::fs; the one filesystem seam is {READER}'s GraphInputReader"
            );
            readers += 1;
        }
    }
    assert_eq!(
        readers, 1,
        "exactly one module may name the filesystem, and {READER} is it"
    );
}

/// The views emit structural facts and carry no score, trust grade or threshold
/// classification.
///
/// Restates "emits structural facts without scores or threshold labels".
///
/// Trace: FR-062-AC-12, FR-062-CON-2
/// Provenance: quoin#500
#[test]
fn tc_500_048_the_views_carry_no_score_or_threshold() {
    let event = json!({ "who": "@reviewer", "commit": REVISION, "note": "still valid" });
    let rendered =
        render_graph_analysis_json(&GraphAnalysis::from(analyze_churn(&plain(&[binding(
            "FR-001-AC-1",
            "unit",
            Some(json!([event])),
        )]))))
        .expect("the report has a canonical spelling");

    let lowered = rendered.to_lowercase();
    for banned in ["trust", "quality", "releasereadiness", "threshold", "score"] {
        assert!(
            !lowered.contains(banned),
            "the view names {banned}; FR-062 reports structure and grades nothing"
        );
    }
    assert!(
        rendered.contains("eventCount"),
        "the structural fact itself must still be there — an empty report would pass the check \
         above by saying nothing"
    );
}
