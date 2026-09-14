// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The governed graph portfolio, byte-for-byte against the retained TypeScript.
//!
//! Trace: FR-067-AC-1
//! Trace: FR-100-AC-4
//! Trace: FR-101-AC-5
//! Provenance: quoin#476
//!
//! # What this gate is
//!
//! `tests/goldens/graph-portfolio-oracle.json` was captured **once** from
//! `src/measurement/graph-portfolio.ts` by
//! `oracle/capture-graph-portfolio-oracle.mjs`, over the committed fixture tree
//! under `tests/fixtures/graph-portfolio-tree/`. This test runs the Rust over
//! the same tree and compares bytes. No node process is spawned here, and
//! `tc_476_boundary.rs` asserts none can be.
//!
//! # Why the inputs are rebuilt rather than replayed
//!
//! `graph-portfolio.ts` is pure: it is handed a portfolio entry, plans, reads
//! and an opaque graph. Replaying a serialised input would compare the Rust
//! against a shape this test wrote. Instead both sides build their inputs from
//! the same committed tree with the same already-ported readers
//! (`read_measurement_collection_results`, `load_measurement_plans`,
//! `build_portfolio_report_from_collections`), and the two things no reader
//! supplies — the opaque FR-062 structural graph and the collection refusals a
//! loader would synthesise — are *data* in `graph-portfolio-tree/graph-inputs.json`,
//! read identically by the capture script and by this file.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::portfolio::{
    PortfolioCollectionSnapshot, PortfolioRepositoryReport, build_portfolio_report_from_collections,
};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::store::read_measurement_collection_results;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement_graph::availability::{CollectionUnavailability, GraphAvailability};
use quoin_measurement_graph::canonical::pretty_text;
use quoin_measurement_graph::error::GraphAdapterErrorCode;
use quoin_measurement_graph::input::{
    GraphCollectionRead, GraphPortfolioRepositoryInput, InjectedStructuralGraph,
};
use quoin_measurement_graph::mapping::{
    GraphPortfolioMappingOptions, ResolvedMapping, parse_graph_portfolio_mappings,
};
use quoin_measurement_graph::reading::GraphCompatibilityCode;
use quoin_measurement_graph::{
    build_governed_graph_portfolio_from, canonical_graph_portfolio_json,
    compare_graph_quality_collections, render_governed_graph_portfolio,
};
use quoin_store::JsonValue;
use serde_json::Value;

/// The token the capture substituted for the fixture tree's absolute root.
const TREE_TOKEN: &str = "@@TREE@@";

/// The floor every case list is measured against. A list that silently
/// emptied would otherwise pass every assertion in this file.
const MAPPING_CASE_FLOOR: usize = 15;

/// As [`MAPPING_CASE_FLOOR`], for the comparison cases.
const COMPARISON_CASE_FLOOR: usize = 7;

/// The lowest number of `unknown` partitions the governed history must name.
///
/// One per governed collection in `alpha`, each from an observation that
/// states no `dimensions` at all. See `DIVERGENCE.md` §2.
const UNKNOWN_PARTITION_FLOOR: usize = 2;

/// As [`MAPPING_CASE_FLOOR`], for the repositories in the portfolio case.
const REPOSITORY_FLOOR: usize = 6;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tree() -> PathBuf {
    crate_root().join("tests/fixtures/graph-portfolio-tree")
}

fn repo(name: &str) -> PathBuf {
    tree().join(name)
}

fn golden() -> Value {
    let path = crate_root().join("tests/goldens/graph-portfolio-oracle.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!("{}: unreadable golden: {error}", path.display());
    });
    serde_json::from_slice(&bytes).unwrap_or_else(|error| {
        panic!("{}: golden is not JSON: {error}", path.display());
    })
}

/// Replace this checkout's fixture root with the captured token, which is the
/// capture's one normalisation and this file's one normalisation.
fn normalise(text: &str) -> String {
    text.replace(&tree().to_string_lossy().into_owned(), TREE_TOKEN)
}

fn text_at<'a>(value: &'a Value, path: &[&str]) -> &'a str {
    let mut cursor = value;
    for step in path {
        cursor = cursor.get(step).unwrap_or_else(|| {
            panic!("golden has no {}", path.join("."));
        });
    }
    cursor.as_str().unwrap_or_else(|| {
        panic!("golden {} is not a string", path.join("."));
    })
}

fn array_at<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("golden has no {key} array"))
        .as_slice()
}

fn paths(value: Option<&Value>) -> Vec<PathBuf> {
    strings(value).into_iter().map(PathBuf::from).collect()
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.replace(TREE_TOKEN, &tree().to_string_lossy()))
                .collect()
        })
        .unwrap_or_default()
}

// ------------------------------------------------------------ the inputs

/// What `graph-inputs.json` states: the injected graphs and refusals.
struct Injected {
    repositories: Vec<String>,
    graphs: Value,
    refusals: Value,
}

fn injected() -> Injected {
    let path = tree().join("graph-inputs.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!("{}: unreadable: {error}", path.display());
    });
    let value: Value = serde_json::from_slice(&bytes).unwrap_or_else(|error| {
        panic!("{}: not JSON: {error}", path.display());
    });
    Injected {
        repositories: array_at(&value, "repositories")
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        graphs: value.get("graphs").cloned().unwrap_or(Value::Null),
        refusals: value.get("refusals").cloned().unwrap_or(Value::Null),
    }
}

fn stored(value: &Value) -> JsonValue {
    from_serde(value).unwrap_or_else(|error| {
        panic!("injected graph member does not cross the bridge: {error}");
    })
}

/// The injected graph for one repository, as `graph-portfolio.ts` would be
/// handed it.
fn graph_of(injected: &Injected, name: &str) -> InjectedStructuralGraph {
    let value = injected
        .graphs
        .get(name)
        .unwrap_or_else(|| panic!("graph-inputs.json states no graph for {name}"));
    let member = |key: &str| value.get(key).cloned();
    let availability = value
        .get("availability")
        .and_then(Value::as_str)
        .and_then(GraphAvailability::from_wire)
        .unwrap_or_else(|| panic!("{name}: unknown injected graph availability"));
    let path = value.get("path").and_then(Value::as_str).map(str::to_owned);
    if availability == GraphAvailability::Available {
        return InjectedStructuralGraph::Available {
            path,
            premises: stored(&member("premises").unwrap_or(Value::Null)),
            fan_out: stored(&member("fanOut").unwrap_or(Value::Null)),
            churn: stored(&member("churn").unwrap_or(Value::Null)),
            change_impact: member("changeImpact").map(|rows| {
                rows.as_array()
                    .unwrap_or_else(|| panic!("{name}: changeImpact is not an array"))
                    .iter()
                    .map(stored)
                    .collect()
            }),
        };
    }
    InjectedStructuralGraph::Unavailable {
        availability: GraphAvailability::from_wire(value["availability"].as_str().unwrap_or(""))
            .and_then(|wide| match wide {
                GraphAvailability::Available => None,
                other => quoin_measurement_graph::availability::GapAvailability::from_wire(
                    other.as_str(),
                ),
            })
            .unwrap_or_else(|| panic!("{name}: injected graph is not a refusal")),
        path,
        reason: value
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    }
}

/// Every read of one repository's store, in the order the retained loader
/// hands them over: the store's own order, then the injected refusals.
fn reads_of(injected: &Injected, name: &str) -> Vec<GraphCollectionRead> {
    let root = repo(name);
    let source = DiskMeasurement::new(&root);
    let mut reads: Vec<GraphCollectionRead> = read_measurement_collection_results(&source)
        .unwrap_or_else(|error| panic!("{name}: store is unreadable: {error}"))
        .into_iter()
        .map(|result| {
            // `result.path` is already the whole path the seam names.
            let path = result.path;
            match result.collection {
                Ok(collection) => GraphCollectionRead::read(path, collection),
                Err(error) => GraphCollectionRead::refused(
                    path,
                    Some(CollectionUnavailability::Unreadable),
                    Some(error.to_string()),
                ),
            }
        })
        .collect();
    for refusal in injected
        .refusals
        .get(name)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        let availability = refusal
            .get("availability")
            .and_then(Value::as_str)
            .and_then(CollectionUnavailability::from_wire);
        reads.push(GraphCollectionRead::refused(
            root.join(refusal["path"].as_str().unwrap_or_default())
                .to_string_lossy()
                .into_owned(),
            availability,
            refusal
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ));
    }
    reads
}

/// `collectionOrder`, as the retained loader sorts a repository's snapshots
/// before handing them to the ungoverned portfolio builder.
fn snapshot_of(reads: &[GraphCollectionRead]) -> Vec<MeasurementCollection> {
    let mut collections: Vec<MeasurementCollection> = reads
        .iter()
        .filter_map(|read| read.collection().cloned())
        .collect();
    collections.sort_by(|left, right| {
        quoin_measurement_graph::order::compare_instants(
            left.timestamp.as_str(),
            right.timestamp.as_str(),
        )
        .then_with(|| {
            quoin_measurement_graph::order::compare_text(
                left.collection_id.as_str(),
                right.collection_id.as_str(),
            )
        })
    });
    collections
}

/// Everything `buildGovernedGraphPortfolioFrom` is handed, built from the tree.
fn inputs() -> Vec<GraphPortfolioRepositoryInput> {
    let injected = injected();
    let reads: Vec<(String, Vec<GraphCollectionRead>)> = injected
        .repositories
        .iter()
        .map(|name| (name.clone(), reads_of(&injected, name)))
        .collect();
    let snapshots: Vec<PortfolioCollectionSnapshot> = reads
        .iter()
        .map(|(name, reads)| PortfolioCollectionSnapshot {
            root: repo(name),
            collections: snapshot_of(reads),
        })
        .collect();
    let portfolio = build_portfolio_report_from_collections(&snapshots);
    let by_root: BTreeMap<&str, &PortfolioRepositoryReport> = portfolio
        .repositories
        .iter()
        .map(|entry| (entry.root.as_str(), entry))
        .collect();
    reads
        .into_iter()
        .map(|(name, collections)| {
            let root = repo(&name);
            let base = by_root
                .get(root.to_string_lossy().as_ref())
                .unwrap_or_else(|| panic!("portfolio omitted resolved repository {name}"));
            GraphPortfolioRepositoryInput {
                portfolio: (*base).clone(),
                plans: load_measurement_plans(
                    &DiskMeasurement::new(&root),
                    PlanLoadOptions {
                        include_governance: true,
                    },
                )
                .unwrap_or_else(|error| panic!("{name}: plans are unreadable: {error}")),
                collections,
                graph: graph_of(&injected, &name),
            }
        })
        .collect()
}

// ------------------------------------------------------------- the gates

/// The governed portfolio's canonical JSON and Markdown are the retained
/// bytes.
///
/// Trace: FR-067-AC-1
/// Trace: FR-100-AC-4
/// Provenance: quoin#476
#[test]
fn tc_476_001_governed_portfolio_is_byte_identical() {
    let golden = golden();
    let case = golden
        .get("portfolio")
        .unwrap_or_else(|| panic!("golden has no portfolio case"));
    let repositories = array_at(case, "repositories");
    assert!(
        repositories.len() >= REPOSITORY_FLOOR,
        "anti-vacuity: the capture covered {} repositories, expected at least {REPOSITORY_FLOOR}",
        repositories.len()
    );

    let inputs = inputs();
    assert_eq!(
        inputs.len(),
        repositories.len(),
        "the tree and the capture disagree on how many repositories there are"
    );
    let report = build_governed_graph_portfolio_from(&inputs)
        .unwrap_or_else(|error| panic!("the governed portfolio refused to build: {error}"));

    assert_eq!(
        normalise(
            &canonical_graph_portfolio_json(&report)
                .unwrap_or_else(|error| panic!("canonical JSON refused: {error}"))
        ),
        text_at(case, &["json"]),
        "canonical governed graph portfolio JSON diverged from the capture"
    );
    assert_eq!(
        normalise(
            &render_governed_graph_portfolio(&report)
                .unwrap_or_else(|error| panic!("the Markdown renderer refused: {error}"))
        ),
        text_at(case, &["rendered"]),
        "the rendered governed graph portfolio diverged from the capture"
    );

    // `DIVERGENCE.md` §2: an observation with no `dimensions` becomes the
    // partition `unknown | unknown | unknown` in both implementations. The
    // bytes above prove they agree; this proves the fallback was reached at
    // all, so the declaration is a measurement rather than a licence.
    let partitions = text_at(case, &["json"])
        .matches("\"measure\": \"unknown\"")
        .count();
    assert!(
        partitions >= UNKNOWN_PARTITION_FLOOR,
        "the capture named {partitions} `unknown` partitions, expected at least \
         {UNKNOWN_PARTITION_FLOOR}: the dimensionless observations left the fixture \
         and DIVERGENCE.md §2 no longer measures anything"
    );
}

/// Every branch of `parseGraphPortfolioMappings`, accepted and refused.
///
/// Trace: FR-067-AC-2
/// Trace: FR-100-AC-4
/// Provenance: quoin#476
#[test]
fn tc_476_002_mappings_resolve_as_the_retained_module_does() {
    let golden = golden();
    let cases = array_at(&golden, "mappings");
    assert!(
        cases.len() >= MAPPING_CASE_FLOOR,
        "anti-vacuity: {} mapping cases captured, expected at least {MAPPING_CASE_FLOOR}",
        cases.len()
    );
    let mut accepted = 0_usize;
    let mut refused = 0_usize;

    for case in cases {
        let name = text_at(case, &["name"]);
        let options = case
            .get("options")
            .unwrap_or_else(|| panic!("{name}: no options"));
        let parsed = parse_graph_portfolio_mappings(
            &paths(case.get("locations")),
            &GraphPortfolioMappingOptions {
                graph_exports: strings(options.get("graphExports")),
                graph_premises: strings(options.get("graphPremises")),
                graph_audits: strings(options.get("graphAudits")),
                changed: strings(options.get("changed")),
                cwd: Some(tree()),
            },
        );
        match (case["verdict"].as_str().unwrap_or_default(), parsed) {
            ("accepted", Ok(mappings)) => {
                accepted += 1;
                assert_eq!(
                    normalise(&mapping_json(&mappings)),
                    text_at(case, &["output"]),
                    "{name}: resolved mappings diverged from the capture"
                );
            }
            ("refused", Err(error)) => {
                refused += 1;
                // The retained union's spelling, which this crate keeps beside
                // its own `QMG-` code precisely so a capture can be compared.
                assert_eq!(
                    error.code().retained_spelling(),
                    case["code"].as_str(),
                    "{name}: refused with a different code"
                );
                assert_eq!(
                    normalise(&format!(
                        "{}: {}",
                        error.code().retained_spelling().unwrap_or_default(),
                        error.message()
                    )),
                    text_at(case, &["message"]),
                    "{name}: refused with a different message"
                );
            }
            ("accepted", Err(error)) => {
                panic!("{name}: refused where the capture accepted: {error}")
            }
            ("refused", Ok(_)) => panic!("{name}: accepted where the capture refused"),
            (verdict, _) => panic!("{name}: unknown captured verdict {verdict}"),
        }
    }
    assert!(
        accepted > 0 && refused > 0,
        "both verdicts must be exercised"
    );
}

/// `compareGraphQualityCollections` over collection pairs, including every
/// blocking compatibility code.
///
/// Trace: FR-067-AC-3
/// Trace: FR-100-AC-4
/// Provenance: quoin#476
#[test]
fn tc_476_003_collection_comparison_is_byte_identical() {
    let golden = golden();
    let cases = array_at(&golden, "comparisons");
    assert!(
        cases.len() >= COMPARISON_CASE_FLOOR,
        "anti-vacuity: {} comparison cases captured, expected at least {COMPARISON_CASE_FLOOR}",
        cases.len()
    );
    let injected = injected();
    let reads: BTreeMap<String, Vec<GraphCollectionRead>> = injected
        .repositories
        .iter()
        .map(|name| (name.clone(), reads_of(&injected, name)))
        .collect();
    let collection = |repository: &str, id: &str| -> MeasurementCollection {
        reads
            .get(repository)
            .unwrap_or_else(|| panic!("{repository}: not in the tree"))
            .iter()
            .filter_map(GraphCollectionRead::collection)
            .find(|row| row.collection_id.as_str() == id)
            .cloned()
            .unwrap_or_else(|| panic!("{repository}: no collection {id}"))
    };

    let mut codes: Vec<String> = Vec::new();
    for case in cases {
        let name = text_at(case, &["name"]);
        let before = collection(
            text_at(case, &["before", "repository"]),
            text_at(case, &["before", "collection"]),
        );
        let after = collection(
            text_at(case, &["after", "repository"]),
            text_at(case, &["after", "collection"]),
        );
        let rows = compare_graph_quality_collections(&before, &after)
            .unwrap_or_else(|error| panic!("{name}: comparison refused: {error}"));
        for row in &rows {
            for reason in &row.reasons {
                codes.push(reason.code.as_str().to_owned());
            }
        }
        assert_eq!(
            normalise(&comparison_json(&rows)),
            text_at(case, &["json"]),
            "{name}: comparison rows diverged from the capture"
        );
    }
    codes.sort_unstable();
    codes.dedup();
    // `collection_incompatible` is the one code this function cannot raise:
    // `compareHistoryPair` adds it *around* the rows, from the availability of
    // the two collections rather than from the observations
    // (`graph-portfolio.ts:497-516`). Every other code is reachable here, and
    // the excluded one is asserted present in the portfolio capture below.
    let mut expected: Vec<&str> = GraphCompatibilityCode::ALL
        .iter()
        .map(|code| code.as_str())
        .filter(|code| *code != GraphCompatibilityCode::CollectionIncompatible.as_str())
        .collect();
    expected.sort_unstable();
    assert_eq!(
        codes, expected,
        "the capture must exercise every blocking code a row comparison can raise"
    );
    assert!(
        text_at(&golden, &["portfolio", "json"])
            .contains(GraphCompatibilityCode::CollectionIncompatible.as_str()),
        "the portfolio capture must exercise the one code a bare row comparison cannot"
    );
}

// ------------------------------------------- the two test-side projections
//
// `graph-portfolio.ts` exports no JSON renderer for a mapping or for a bare
// comparison row; the capture wrote them with `canonicalJson`. These functions
// are the same shapes, spelled here rather than in the crate, so no wire view
// exists in the library that the retained module does not have. Both go
// through [`pretty_text`], the crate's one canonical writer.

fn mapping_json(mappings: &[ResolvedMapping]) -> String {
    let rows: Vec<Value> = mappings
        .iter()
        .map(|mapping| {
            let root = display(mapping.root());
            let changed: Vec<Value> = mapping
                .changed()
                .iter()
                .map(|seed| Value::String(seed.clone()))
                .collect();
            match *mapping {
                ResolvedMapping::Ready {
                    ref export_path,
                    ref premises_path,
                    ref audit_path,
                    ..
                } => serde_json::json!({
                    "root": root,
                    "status": "ready",
                    "exportPath": display(export_path),
                    "premisesPath": display(premises_path),
                    "auditPath": display(audit_path),
                    "changed": changed,
                }),
                ResolvedMapping::Refused { status, .. } => serde_json::json!({
                    "root": root,
                    "status": status.as_str(),
                    "reason": mapping.reason().unwrap_or_default(),
                    "changed": changed,
                }),
            }
        })
        .collect();
    pretty_text(&Value::Array(rows)).unwrap_or_else(|error| panic!("mapping JSON refused: {error}"))
}

fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn comparison_json(rows: &[quoin_measurement_graph::reading::GraphQualityComparisonRow]) -> String {
    let rows: Vec<Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "measure": row.identity.measure,
                "dimension": row.identity.dimension,
                "key": row.identity.key,
                "before": row.before,
                "after": row.after,
                "delta": row.delta,
                "status": row.status.as_str(),
                "reasons": row
                    .reasons
                    .iter()
                    .map(|reason| serde_json::json!({
                        "code": reason.code.as_str(),
                        "blocking": quoin_measurement_graph::reading::GraphCompatibilityReason::BLOCKING,
                        "message": reason.message,
                    }))
                    .collect::<Vec<Value>>(),
            })
        })
        .collect();
    pretty_text(&Value::Array(rows))
        .unwrap_or_else(|error| panic!("comparison JSON refused: {error}"))
}

/// The error vocabulary the mapping cases reach is the crate's, not a second
/// one bolted on for the test.
///
/// Trace: FR-100-AC-8
/// Provenance: quoin#476
#[test]
fn tc_476_004_mapping_codes_are_the_crate_vocabulary() {
    let golden = golden();
    let captured: Vec<&str> = array_at(&golden, "mappings")
        .iter()
        .filter_map(|case| case.get("code").and_then(Value::as_str))
        .collect();
    assert!(
        !captured.is_empty(),
        "anti-vacuity: no refusal was captured at all"
    );
    for code in &captured {
        assert!(
            GraphAdapterErrorCode::ALL
                .iter()
                .any(|known| known.retained_spelling() == Some(*code)),
            "the capture refused with {code}, which this crate does not spell"
        );
    }
}
