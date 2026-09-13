// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What `src/graph-analysis/` computed, this crate computes.
//!
//! The corpus in `tests/goldens/graph-analysis.json` was captured **once**,
//! by `oracle/capture-graph-analysis.mjs`, from the retained TypeScript at the
//! revision the capture records. It is committed, and the TypeScript is never
//! run here: FR-101-AC-5 forbids leaving an oracle in the loop, and a test
//! that shelled out to node would be exactly that.
//!
//! # What is compared, and how exactly
//!
//! * **The three views.** For every case, `renderGraphAnalysisJson` and
//!   `renderGraphAnalysis` are compared **byte for byte**. The JSON side is
//!   the acceptance criterion the ticket states, and the bytes come from
//!   `quoin_store::canonical_json`; the markdown side is compared too, because
//!   the row order it prints is the sort under test made visible.
//! * **The input contracts.** Accepted or refused, per named text. When the
//!   oracle accepted, the value it parsed is compared as well.
//! * **The loader.** Accepted or refused, which input the refusal names, and
//!   which of the three bindings-store states it reached.
//!
//! # What is deliberately not compared, and why
//!
//! **Refusal wording.** The retained refusals are zod's prose; these are this
//! crate's own sentences. quoin#403 rules that diagnostic text is not
//! contractual, and `quoin-jsonschema`'s `tc_470` already relies on that
//! ruling. The *verdict* and the *named input* are contractual and are both
//! asserted. The corpus does not record the retained wording at all, so there
//! is nothing here that could silently drift.
//!
//! **Member order inside a parsed contract value.** The `canonical` the
//! capture recorded is `JSON.stringify(parsed.value)`, which emits members in
//! JavaScript insertion order. No surface of this crate emits that order:
//! every byte it writes goes through canonical JSON, which sorts members at
//! every level. So both sides are normalised through
//! `quoin_store::canonicalize_jcs` before comparison, which keeps **array**
//! order — the sorting that `canonicalizeAuditReport` and
//! `canonicalizeAcceptedAssurancePremises` actually decide — under test while
//! dropping an order nobody can observe.
//!
//! # Divergences are declared, and must fire
//!
//! [`DECLARED_DIVERGENCES`] names every case where this crate deliberately
//! disagrees with the capture. An undeclared disagreement fails, and a
//! declared one that has stopped disagreeing fails too: a divergence nothing
//! exercises is an unmeasured claim.
//!
//! Trace: FR-062-AC-1, FR-062-AC-2, FR-062-AC-3, FR-101-AC-5
//!
//! Provenance: quoin#385

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_graph_analysis::model::binding::BindingInput;
use quoin_graph_analysis::{
    ArtifactId, GraphAnalysis, GraphErrorCode, GraphInput, GraphInputReader, GraphLoadOptions,
    RelationKind, analyze_change_impact, analyze_churn, analyze_fan_out, check_accepted_premises,
    check_audit_identity, load_graph_analysis_input, parse_accepted_premises,
    parse_assurance_export, parse_audit_envelope, render_graph_analysis,
    render_graph_analysis_json,
};
use serde_json::Value;

/// The counts below which this parity measurement is measuring nothing.
const CASE_FLOOR: usize = 20;
/// As [`CASE_FLOOR`], for the input-contract corpus.
const CONTRACT_FLOOR: usize = 25;
/// As [`CASE_FLOOR`], for the loader corpus.
const LOAD_FLOOR: usize = 12;

/// Where this crate deliberately disagrees with the capture.
///
/// `(case id, what disagrees, the ruling)`. One entry.
///
/// `stableKey` (`input.ts:274`) is `JSON.stringify(canonical(value))`, and
/// `canonical` sorts member names before rebuilding the object. A JavaScript
/// object does not keep that order: a name that is an *array index* is emitted
/// first, in ascending numeric order, whatever order it was inserted in. So
/// the retained sort key is RFC 8785 JCS for every document except one that
/// carries an array-index member name, where the two orders part company.
///
/// This crate calls `quoin_store::canonicalize_jcs` rather than growing a
/// third serializer to reproduce an ECMAScript property-order rule. The one
/// corpus rung that can see the difference is
/// `audit/array-index-passthrough-order`: two findings alike but for
/// passthrough members named `2` and `10`, whose sort keys order them one way
/// under `JSON.stringify` and the other under JCS.
const DECLARED_DIVERGENCES: &[(&str, &str, &str)] = &[(
    "audit/array-index-passthrough-order",
    "the order of two findings whose sort keys differ only in array-index-named passthrough \
     members",
    "quoin#385: `stableKey` is RFC 8785 JCS except where ECMAScript array-index property order \
     applies; this crate uses `quoin_store::canonicalize_jcs` rather than reimplement that rule",
)];

/// One case's three views, each as `(view name, json, markdown)`.
fn rendered(case: &Value) -> Vec<(&'static str, String, String)> {
    let id = case["id"].as_str().expect("a case id");
    let input = support::input(case);
    let requested: Vec<ArtifactId> = case["requested"]
        .as_array()
        .expect("requested is an array")
        .iter()
        .map(|value| ArtifactId::new(value.as_str().expect("an artifact id")))
        .collect();
    let relations: Option<Vec<RelationKind>> = case["relations"].as_array().map(|kinds| {
        kinds
            .iter()
            .map(|value| RelationKind::new(value.as_str().expect("a relation kind")))
            .collect()
    });
    let impact = analyze_change_impact(&input, &requested, relations.as_deref())
        .unwrap_or_else(|error| panic!("{id}: change impact is computable: {error}"));

    [
        ("fan-out", GraphAnalysis::from(analyze_fan_out(&input))),
        ("churn", GraphAnalysis::from(analyze_churn(&input))),
        ("change-impact", GraphAnalysis::from(impact)),
    ]
    .into_iter()
    .map(|(view, analysis)| {
        let json = render_graph_analysis_json(&analysis)
            .unwrap_or_else(|error| panic!("{id}/{view}: the report is canonicalizable: {error}"));
        (view, json, render_graph_analysis(&analysis))
    })
    .collect()
}

/// Every case's three views render the captured canonical JSON, byte for byte.
///
/// Trace: FR-062-AC-1
#[test]
fn tc_385_010_the_views_render_the_captured_canonical_json() {
    let corpus = support::corpus();
    let cases = corpus["cases"].as_array().expect("cases is an array");
    assert!(
        cases.len() >= CASE_FLOOR,
        "anti-vacuity floor: at least {CASE_FLOOR} analysis cases expected, saw {}",
        cases.len()
    );
    for case in cases {
        let id = case["id"].as_str().expect("a case id");
        for (view, json, _) in rendered(case) {
            assert_eq!(
                json,
                case["views"][view]["json"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{id}/{view}: the capture records json")),
                "{id}/{view}: the canonical JSON is not the bytes the retained renderer wrote"
            );
        }
    }
}

/// Every case's three views render the captured markdown, byte for byte.
///
/// Trace: FR-062-AC-1
#[test]
fn tc_385_011_the_views_render_the_captured_markdown() {
    let corpus = support::corpus();
    for case in corpus["cases"].as_array().expect("cases is an array") {
        let id = case["id"].as_str().expect("a case id");
        for (view, _, markdown) in rendered(case) {
            assert_eq!(
                markdown,
                case["views"][view]["markdown"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{id}/{view}: the capture records markdown")),
                "{id}/{view}: the markdown is not what the retained renderer wrote"
            );
        }
    }
}

/// One contract case's verdict, and the value it parsed when it was accepted.
fn contract_verdict(entry: &Value) -> (bool, Option<String>) {
    let id = entry["id"].as_str().expect("a case id");
    let text = entry["text"].as_str().expect("a case text");
    let against = || {
        parse_assurance_export(
            entry["against_export"]
                .as_str()
                .expect("a cross-check records its export"),
        )
        .unwrap_or_else(|error| panic!("{id}: the captured export is valid: {error}"))
    };
    match entry["contract"].as_str() {
        Some("premises") => match parse_accepted_premises(text) {
            Ok(premises) => (true, Some(jcs(id, &premises))),
            Err(_) => (false, None),
        },
        Some("audit") => match parse_audit_envelope(text) {
            Ok(envelope) => (true, Some(jcs(id, &envelope))),
            Err(_) => (false, None),
        },
        Some("premises-match") => {
            let premises = parse_accepted_premises(text)
                .unwrap_or_else(|error| panic!("{id}: the candidate premises parse: {error}"));
            (check_accepted_premises(&against(), &premises).is_ok(), None)
        }
        Some("audit-identity") => {
            let envelope = parse_audit_envelope(text)
                .unwrap_or_else(|error| panic!("{id}: the candidate envelope parses: {error}"));
            (check_audit_identity(&envelope, &against()).is_ok(), None)
        }
        other => panic!("{id}: unknown contract {other:?}"),
    }
}

/// One value as RFC 8785 JCS, through the one canonicalizer this crate uses.
fn jcs<T: serde::Serialize>(id: &str, value: &T) -> String {
    let text = serde_json::to_string(value).expect("a parsed contract value serialises");
    let parsed = quoin_store::parse_strict_json_str(&text)
        .unwrap_or_else(|error| panic!("{id}: the parsed value is I-JSON: {error}"));
    quoin_store::canonicalize_jcs(&parsed)
        .unwrap_or_else(|error| panic!("{id}: the parsed value canonicalizes: {error}"))
}

/// The input contracts reach the captured verdicts and the captured values.
///
/// Trace: FR-062-AC-2
#[test]
fn tc_385_012_the_input_contracts_reach_the_captured_verdicts() {
    let corpus = support::corpus();
    let entries = corpus["contract"].as_array().expect("contract is an array");
    assert!(
        entries.len() >= CONTRACT_FLOOR,
        "anti-vacuity floor: at least {CONTRACT_FLOOR} contract cases expected, saw {}",
        entries.len()
    );

    let mut fired: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in entries {
        let id = entry["id"].as_str().expect("a case id");
        let expected = entry["accepted"].as_bool().expect("a captured verdict");
        let (accepted, canonical) = contract_verdict(entry);
        assert_eq!(
            accepted, expected,
            "{id}: the retained contract said accepted={expected} and this crate said \
             accepted={accepted}. The verdict is contractual; the wording is not."
        );
        let Some(captured) = entry["canonical"].as_str() else {
            continue;
        };
        let captured = jcs(
            id,
            &serde_json::from_str::<Value>(captured).expect("captured JSON"),
        );
        let mine = canonical.expect("an accepted contract yields a value");
        if let Some((name, what, ruling)) = DECLARED_DIVERGENCES.iter().find(|(n, ..)| *n == id) {
            assert_ne!(
                mine, captured,
                "the declared divergence {name} ({what}, {ruling}) no longer fires: the two \
                 implementations now agree, so the declaration is stale and must be removed"
            );
            *fired.entry(name).or_insert(0) += 1;
            continue;
        }
        assert_eq!(
            mine, captured,
            "{id}: the parsed value is not the one the retained contract produced. If the \
             difference is intended it belongs in DECLARED_DIVERGENCES; otherwise it is a port \
             defect."
        );
    }

    for (name, ..) in DECLARED_DIVERGENCES {
        assert!(
            fired.get(name).is_some_and(|count| *count > 0),
            "the declared divergence {name} was never exercised: the corpus no longer carries \
             the case it was recorded against, so it is an unmeasured claim"
        );
    }
}

/// A tree of files, and nothing else, standing in for the filesystem.
struct FakeTree(BTreeMap<PathBuf, String>);

impl GraphInputReader for FakeTree {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        self.0.get(path).cloned().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("{}", path.display()))
        })
    }
}

/// The loader reaches the captured outcome for every recorded tree.
///
/// Trace: FR-062-AC-3
#[test]
fn tc_385_013_the_loader_reaches_the_captured_outcomes() {
    let corpus = support::corpus();
    let entries = corpus["load"].as_array().expect("load is an array");
    assert!(
        entries.len() >= LOAD_FLOOR,
        "anti-vacuity floor: at least {LOAD_FLOOR} loader cases expected, saw {}",
        entries.len()
    );

    for entry in entries {
        let id = entry["id"].as_str().expect("a case id");
        let options = GraphLoadOptions {
            repo: PathBuf::from("/repo"),
            export_path: PathBuf::from("/in/export.json"),
            premises_path: PathBuf::from("/in/premises.json"),
            audit_path: PathBuf::from("/in/audit.json"),
        };
        let mut tree = BTreeMap::new();
        for (path, text) in [
            (options.export_path.clone(), &entry["export_text"]),
            (options.premises_path.clone(), &entry["premises_text"]),
            (options.audit_path.clone(), &entry["audit_text"]),
            (options.bindings_path(), &entry["bindings_file"]),
        ] {
            if let Some(text) = text.as_str() {
                tree.insert(path, text.to_owned());
            }
        }

        let outcome = load_graph_analysis_input(&FakeTree(tree), &options);
        match (entry["ok"].as_bool().expect("a captured verdict"), outcome) {
            (true, Ok(loaded)) => {
                let availability = match loaded.bindings {
                    BindingInput::Available(_) => "available",
                    BindingInput::Absent { .. } => "absent",
                    BindingInput::Unreadable { .. } => "unreadable",
                };
                assert_eq!(
                    Some(availability),
                    entry["bindings_availability"].as_str(),
                    "{id}: the bindings store reached a different state than the retained loader"
                );
            }
            (false, Err(error)) => {
                let kind = match error.code() {
                    GraphErrorCode::InputUnreadable => "graph-input-read",
                    GraphErrorCode::InputInvalid => "graph-input-invalid",
                    GraphErrorCode::Canonicalization => "graph-canonicalization",
                    // `GraphErrorCode` is `#[non_exhaustive]`: a code added
                    // later has no captured spelling, and saying so is better
                    // than mapping it onto one of the three that do.
                    other => panic!("{id}: {other} has no retained loader kind"),
                };
                assert_eq!(
                    Some(kind),
                    entry["error_kind"].as_str(),
                    "{id}: the refusal is of a different kind than the retained loader's"
                );
                assert_eq!(
                    error.input().map(GraphInput::as_str),
                    entry["error_input"].as_str(),
                    "{id}: the refusal names a different input than the retained loader's"
                );
            }
            (expected, actual) => panic!(
                "{id}: the retained loader said ok={expected}; this crate said {}",
                if actual.is_ok() { "ok" } else { "refused" }
            ),
        }
    }
}

/// The corpus separates the verdicts it is supposed to separate.
///
/// A contract corpus that refused everything would agree with any parser that
/// refuses everything, and a loader corpus that never reached an absent store
/// would say nothing about the three-state read.
///
/// Trace: FR-062-AC-2
#[test]
fn tc_385_014_the_corpus_carries_more_than_one_answer() {
    let corpus = support::corpus();
    let contract = corpus["contract"].as_array().expect("contract is an array");
    let accepted = contract
        .iter()
        .filter(|entry| entry["accepted"].as_bool() == Some(true))
        .count();
    assert!(
        accepted > 0 && accepted < contract.len(),
        "{accepted} of {} contract cases were accepted; a corpus that is all one verdict cannot \
         separate two implementations",
        contract.len()
    );

    let load = corpus["load"].as_array().expect("load is an array");
    for state in ["available", "absent", "unreadable"] {
        assert!(
            load.iter()
                .any(|entry| entry["bindings_availability"].as_str() == Some(state)),
            "no loader case reaches a {state} bindings store; the three-state read is untested"
        );
    }
    for input in ["export", "premises", "audit"] {
        assert!(
            load.iter()
                .any(|entry| entry["error_input"].as_str() == Some(input)),
            "no loader case refuses on the {input} input"
        );
    }
}

/// The capture says what it ran, and at which revision.
///
/// Trace: FR-101-AC-11
#[test]
fn tc_385_015_the_capture_records_its_provenance() {
    let corpus = support::corpus();
    for key in ["producer", "oracle", "node_version", "quoin_revision"] {
        let value = corpus["provenance"][key].as_str();
        assert!(
            value.is_some_and(|text| !text.is_empty()),
            "the capture must record {key}; a golden whose origin is unrecorded cannot be \
             re-derived"
        );
    }
}
