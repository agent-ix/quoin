// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-067 acceptance criterion the deleted `tests/graph-portfolio.test.ts`
//! carried, restated against this crate, plus StR-007-VC-1.
//!
//! Trace: FR-067-AC-4, FR-067-AC-5, FR-067-AC-6, FR-067-AC-7, FR-067-AC-8
//! Trace: FR-067-AC-9, FR-067-AC-10, FR-067-AC-11
//! Trace: StR-007-VC-1
//! Provenance: quoin#480
//!
//! # Why this file exists
//!
//! `tests/graph-portfolio.test.ts` was deleted in the same commit as
//! `src/measurement/graph-portfolio.ts` (FR-101). `tc_476_graph_portfolio.rs`
//! proves this crate's report is the retained module's bytes over one committed
//! tree, and is tagged `FR-067-AC-1`, `-AC-2` and `-AC-3` for the three
//! properties that tree exercises. The other eight criteria had no Rust home.
//! They do now, one test each, named by the criterion.
//!
//! # Where the inputs come from
//!
//! Built here, the way the deleted tests built theirs: a template collection
//! mutated per criterion and admitted through
//! [`quoin_measurement::validate::stored_measurement_collection`], so nothing
//! in this file asserts against a shape only this file can produce. No node
//! process is spawned; `tc_476_boundary.rs` asserts none can be.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use quoin_measurement::MeasurementPlan;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::portfolio::{
    PortfolioCollectionSnapshot, PortfolioRepositoryReport, build_portfolio_report_from_collections,
};
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::observation::MeasurementState;
use quoin_measurement::types::plan::{LifecycleStatus, MeasurementStage};
use quoin_measurement_graph::availability::{
    CollectionUnavailability, GapAvailability, MappingRefusal,
};
use quoin_measurement_graph::error::GraphAdapterErrorCode;
use quoin_measurement_graph::input::{
    ChangeImpact, ChangeImpactAbsence, NormalizedStructuralGraph,
};
use quoin_measurement_graph::mapping::{GraphPortfolioMappingOptions, ResolvedMapping};
use quoin_measurement_graph::reading::{
    ComparisonStatus, GraphCompatibilityCode, GraphCompatibilityReason,
};
use quoin_measurement_graph::report::GovernedGraphPortfolioReport;
use quoin_measurement_graph::{
    GraphCollectionRead, GraphPortfolioRepositoryInput, InjectedStructuralGraph,
    build_governed_graph_portfolio_from, canonical_graph_portfolio_json,
    compare_graph_quality_collections, parse_graph_portfolio_mappings,
    render_governed_graph_portfolio,
};
use serde_json::{Value, json};

/// Each compatibility premise, and the mutation that breaks exactly it.
type Mutation = (GraphCompatibilityCode, fn(&mut Value));

/// The retained scorer bytes every template collection carries, and the digest
/// they actually hash to — `scorer.rs` re-checks it, so the pair must be real.
const SCORER_BASE64: &str =
    "eyJzY29yZXMiOlswLjgsMC45LDAuNV0sIm5vdGUiOiJncmFwaCBzY29yZXIgb3V0cHV0In0=";
const SCORER_DIGEST: &str =
    "sha256:35977f285a45f1c57dbb3b437bb61c3ac788011232225eec65d0bdb6a9309c47";

/// The active plan every repository in this file governs its evidence by.
fn plan() -> MeasurementPlan {
    let text = |value: &str| -> quoin_measurement::types::ids::NonEmptyText {
        quoin_measurement::types::ids::NonEmptyText::parse(
            value,
            quoin_measurement::MeasurementErrorCode::PlanInvalid,
            "plan",
        )
        .expect("a non-empty plan member")
    };
    MeasurementPlan {
        id: text("gp-graph"),
        title: text("Graph quality"),
        status: LifecycleStatus::Active,
        stage: MeasurementStage::Observe,
        metric: text("graph_quality"),
        definition_version: text("gq-1"),
        path: "spec/assurance/10-graph.md".to_owned(),
        owner: Some("assurance-team".to_owned()),
        action: Some("repair retained evidence".to_owned()),
        preregistration: None,
        ground_truth_kind: None,
        statistical_design: None,
    }
}

/// One template collection, as the deleted tests' `collection()` helper built
/// one: two measured partitions, the producer tuple, and the retained scorer.
fn collection_json(id: &str, timestamp: &str) -> Value {
    json!({
        "schemaVersion": 1,
        "collectionId": id,
        "subject": "quoin",
        "toolIdentity": "agent-ix/quire-code-rs",
        "toolVersion": "1.0.0",
        "configDigest": "sha256:cfg-one",
        "timestamp": timestamp,
        "sourceRevision": "d".repeat(40),
        "corpusRevision": "e".repeat(40),
        "scope": { "corpus": "spec" },
        "environment": { "ci": "true" },
        "observations": [
            {
                "metric": "graph_quality",
                "planId": "gp-graph",
                "definitionVersion": "gq-1",
                "state": "measured",
                "value": 0.5,
                "unit": "ratio",
                "shape": "ratio",
                "population": {
                    "examined": 2,
                    "matched": 2,
                    "complete": true,
                    "identity": { "files": ["a.rs", "b.rs"] },
                },
                "dimensions": {
                    "measure": "census",
                    "dimension": "node_kind",
                    "key": "function",
                },
            },
            {
                "metric": "graph_quality",
                "planId": "gp-graph",
                "definitionVersion": "gq-1",
                "state": "measured",
                "value": 0.25,
                "unit": "ratio",
                "shape": "ratio",
                "population": {
                    "examined": 2,
                    "matched": 2,
                    "complete": true,
                    "identity": { "files": ["a.rs", "b.rs"] },
                },
                "dimensions": {
                    "measure": "census",
                    "dimension": "language",
                    "key": "rust",
                },
            },
        ],
        "rawEvidence": {
            "producer": {
                "observation_id": format!("sha256:{}", format!("{id:0<64}").chars().take(64).collect::<String>()),
                "producer": { "extractor_revision": "v1.2.3", "scorer_version": "v0.4.0" },
                "population": { "state": "measured" },
                "raw_scorer_output": { "digest": SCORER_DIGEST, "path": "out/scores.json" },
            },
            "scorer": {
                "digest": SCORER_DIGEST,
                "bytesBase64": SCORER_BASE64,
                "mediaType": "application/json",
            },
        },
    })
}

/// Admit a template document as a retained collection.
fn admit(value: &Value) -> MeasurementCollection {
    let stored = from_serde(value).expect("the template crosses the bridge");
    quoin_measurement::validate::stored_measurement_collection(&stored)
        .unwrap_or_else(|error| panic!("the template collection is admissible: {error}"))
}

/// A template collection, with `mutate` applied to its JSON first.
fn collection_with(
    id: &str,
    timestamp: &str,
    mutate: impl FnOnce(&mut Value),
) -> MeasurementCollection {
    let mut value = collection_json(id, timestamp);
    mutate(&mut value);
    admit(&value)
}

/// A template collection, unmutated.
fn collection(id: &str, timestamp: &str) -> MeasurementCollection {
    admit(&collection_json(id, timestamp))
}

/// A graph the caller did not load.
fn no_graph() -> InjectedStructuralGraph {
    InjectedStructuralGraph::Unavailable {
        availability: GapAvailability::Missing,
        path: None,
        reason: "no graph export".to_owned(),
    }
}

/// One repository's inputs: its collections, the active plan, and a graph.
fn repository(
    root: &str,
    collections: Vec<MeasurementCollection>,
    reads: Vec<GraphCollectionRead>,
    graph: InjectedStructuralGraph,
) -> GraphPortfolioRepositoryInput {
    let snapshot = PortfolioCollectionSnapshot {
        root: PathBuf::from(root),
        collections: collections.clone(),
    };
    let portfolio = build_portfolio_report_from_collections(&[snapshot]);
    let base: PortfolioRepositoryReport = portfolio
        .repositories
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{root}: the ungoverned portfolio omitted the repository"));
    let mut all = reads;
    all.extend(collections.into_iter().map(|value| {
        let path = format!(
            "{root}/spec/evidence/measurements/{}.json",
            value.collection_id.as_str()
        );
        GraphCollectionRead::read(path, value)
    }));
    GraphPortfolioRepositoryInput {
        portfolio: base,
        plans: vec![plan()],
        collections: all,
        graph,
    }
}

/// The simple case: one repository, these collections, no graph.
fn one(root: &str, collections: Vec<MeasurementCollection>) -> GraphPortfolioRepositoryInput {
    repository(root, collections, Vec::new(), no_graph())
}

/// Build, refusing to hide a refusal.
fn build(inputs: &[GraphPortfolioRepositoryInput]) -> GovernedGraphPortfolioReport {
    build_governed_graph_portfolio_from(inputs)
        .unwrap_or_else(|error| panic!("the governed projection refused: {error}"))
}

// ------------------------------------------------------------- the gates

/// Availability is a separate axis from measurement state, and neither is a
/// numeric zero.
///
/// Trace: FR-067-AC-4
/// Provenance: quoin#480
#[test]
fn tc_480_040_availability_is_separate_from_state_and_from_numeric_zero() {
    for availability in GapAvailability::ALL {
        let report = build(&[repository(
            "/repos/a",
            Vec::new(),
            Vec::new(),
            InjectedStructuralGraph::Unavailable {
                availability: *availability,
                path: None,
                reason: availability.as_str().to_owned(),
            },
        )]);
        let graph = &report.repositories[0].graph;
        assert_eq!(
            graph.availability(),
            availability.as_str(),
            "the injected refusal is the reported one"
        );
        let rendered = canonical_graph_portfolio_json(&report).expect("canonical JSON");
        assert!(
            !rendered.contains("\"value\":0"),
            "{}: an unavailable graph became a measured zero",
            availability.as_str()
        );
    }
    assert!(
        GapAvailability::ALL.len() >= 5,
        "every refusal spelling is measured, not a subset"
    );

    // A not-computed observation stays not computed and carries no value.
    let report = build(&[one(
        "/repos/a",
        vec![collection_with("state", "2026-08-31T00:00:00Z", |value| {
            let row = &mut value["observations"][0];
            row["state"] = Value::String("not_computed".to_owned());
            row["value"] = Value::Null;
            row["reason"] = Value::String("unsupported".to_owned());
        })],
    )]);
    let current = report.repositories[0]
        .graph_quality
        .current
        .as_ref()
        .expect("a current reading");
    let partition = current
        .partitions
        .iter()
        .find(|row| row.state == MeasurementState::NotComputed)
        .expect("the not-computed partition survives");
    assert_eq!(
        partition.value, None,
        "a not-computed partition has no value"
    );
}

/// Every graph compatibility premise blocks a delta on its own.
///
/// Trace: FR-067-AC-5
/// Provenance: quoin#480
#[test]
fn tc_480_041_every_compatibility_premise_blocks_a_delta_independently() {
    let before = collection("before", "2026-01-01T00:00:00Z");
    let comparable =
        compare_graph_quality_collections(&before, &collection("after", "2026-02-01T00:00:00Z"))
            .expect("two template collections compare");
    assert!(
        !comparable.is_empty(),
        "a comparison over no rows compares nothing"
    );
    assert!(
        comparable
            .iter()
            .all(|row| row.status == ComparisonStatus::Comparable && row.delta == Some(0.0)),
        "two identical readings are comparable with a zero delta"
    );
    const {
        assert!(
            GraphCompatibilityReason::BLOCKING,
            "every stated compatibility reason blocks; there is no advisory one"
        );
    }

    let mutations: [Mutation; 7] = [
        (GraphCompatibilityCode::PlanChanged, |value| {
            value["observations"][0]["planId"] = Value::String("gp-other".to_owned());
        }),
        (GraphCompatibilityCode::DefinitionChanged, |value| {
            value["observations"][0]["definitionVersion"] = Value::String("gq-2".to_owned());
        }),
        (GraphCompatibilityCode::ConfigurationChanged, |value| {
            value["configDigest"] = Value::String("sha256:changed".to_owned());
        }),
        (GraphCompatibilityCode::ToolChanged, |value| {
            value["toolVersion"] = Value::String("changed".to_owned());
        }),
        (GraphCompatibilityCode::CorpusChanged, |value| {
            value["corpusRevision"] = Value::String("changed".to_owned());
        }),
        (GraphCompatibilityCode::PopulationChanged, |value| {
            value["observations"][0]["population"]["identity"] = json!({ "files": ["other.rs"] });
        }),
        (GraphCompatibilityCode::PopulationIncomplete, |value| {
            value["observations"][0]["population"]["complete"] = Value::Bool(false);
        }),
    ];

    for (code, mutate) in mutations {
        let after = collection_with(
            &format!("after-{}", code.as_str()),
            "2026-02-01T00:00:00Z",
            mutate,
        );
        let rows = compare_graph_quality_collections(&before, &after).expect("a comparison");
        let row = rows
            .iter()
            .find(|row| row.status != ComparisonStatus::Comparable)
            .unwrap_or_else(|| panic!("{}: nothing was blocked", code.as_str()));
        assert_eq!(
            row.delta,
            None,
            "{}: a blocked row carries no delta",
            code.as_str()
        );
        assert!(
            row.reasons.iter().any(|reason| reason.code == code),
            "{}: the reason names the premise that broke, got {:?}",
            code.as_str(),
            row.reasons
                .iter()
                .map(|reason| reason.code)
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(
        mutations.len() + 1,
        GraphCompatibilityCode::ALL.len(),
        "every code but `collection_incompatible` is a premise a mutation can break"
    );

    retired_plan_evidence_is_reported_incomparable();
}

/// Evidence taken under a plan the repository no longer runs is compared by
/// nothing, and the incompatibility is stated rather than silently dropped.
fn retired_plan_evidence_is_reported_incomparable() {
    // Evidence taken under a plan the repository no longer runs is compared by
    // nothing, and the incompatibility is stated rather than silently dropped.
    let retired = |id: &str, timestamp: &str| {
        collection_with(id, timestamp, |value| {
            for row in value["observations"].as_array_mut().expect("observations") {
                row["planId"] = Value::String("gp-retired".to_owned());
            }
        })
    };
    let report = build(&[one(
        "/repos/retired",
        vec![
            retired("retired-after", "2026-04-01T00:00:00Z"),
            retired("retired-before", "2026-03-01T00:00:00Z"),
        ],
    )]);
    let comparison = report.repositories[0]
        .graph_quality
        .comparison
        .as_ref()
        .expect("two readings are still compared");
    assert!(
        !comparison.observations.is_empty(),
        "an incompatible comparison is reported, not omitted"
    );
    assert!(
        comparison.observations.iter().all(|row| {
            row.status != ComparisonStatus::Comparable
                && row.delta.is_none()
                && row
                    .reasons
                    .iter()
                    .any(|reason| reason.code == GraphCompatibilityCode::CollectionIncompatible)
        }),
        "every row states the collection is incompatible and carries no delta"
    );
    assert!(
        report.repositories[0].graph_quality.current.is_none(),
        "evidence under a retired plan is not a current reading"
    );
}

/// The retained producer and scorer identities reach every view.
///
/// Trace: FR-067-AC-6
/// Provenance: quoin#480
#[test]
fn tc_480_042_raw_identities_reach_every_view() {
    let report = build(&[one(
        "/repos/a",
        vec![
            collection("one", "2026-01-01T00:00:00Z"),
            collection("two", "2026-02-01T00:00:00Z"),
        ],
    )]);
    let quality = &report.repositories[0].graph_quality;
    assert_eq!(quality.history.len(), 2, "both readings are historical");
    for row in &quality.history {
        assert_eq!(
            row.scorer_digest.as_deref(),
            Some(SCORER_DIGEST),
            "{}: the history row carries the retained scorer digest",
            row.id
        );
        assert!(
            row.producer_record_digest
                .as_deref()
                .is_some_and(|digest| digest.starts_with("sha256:")),
            "{}: the history row carries the producer record digest",
            row.id
        );
    }
    let comparison = quality.comparison.as_ref().expect("a comparison");
    assert_eq!(
        comparison.before.scorer_digest.as_deref(),
        Some(SCORER_DIGEST)
    );
    assert!(
        comparison
            .after
            .producer_record_digest
            .as_deref()
            .is_some_and(|digest| digest.starts_with("sha256:")),
        "the compared reading carries its producer identity"
    );
    let current = quality.current.as_ref().expect("a current reading");
    assert_eq!(current.scorer_digest.as_deref(), Some(SCORER_DIGEST));
}

/// Structural report objects are carried unchanged, and an absent input is
/// stated rather than implied.
///
/// Trace: FR-067-AC-7
/// Provenance: quoin#480
#[test]
fn tc_480_043_structural_objects_are_carried_and_absences_are_explicit() {
    let fan_out_value = json!({ "rows": [{ "id": "FR-001", "count": 2 }], "type": "fan-out" });
    let churn_value = json!({ "availability": "unknown", "type": "churn" });
    let impact_value = json!({ "affected": ["TC-001"], "seed": "FR-001", "type": "change-impact" });
    let stored = |value: &Value| from_serde(value).expect("an opaque analysis crosses the bridge");

    let graph = InjectedStructuralGraph::Available {
        path: Some("/repos/a/graph/export.json".to_owned()),
        premises: stored(&json!({ "revision": "a" })),
        fan_out: stored(&fan_out_value),
        churn: stored(&churn_value),
        change_impact: Some(vec![stored(&impact_value)]),
    };
    let report = build(&[repository("/repos/a", Vec::new(), Vec::new(), graph)]);
    match report.repositories[0].graph {
        NormalizedStructuralGraph::Available {
            ref path,
            ref premises,
            ref fan_out,
            ref churn,
            ref change_impact,
        } => {
            assert_eq!(path.as_deref(), Some("/repos/a/graph/export.json"));
            assert_eq!(*premises, stored(&json!({ "revision": "a" })));
            assert_eq!(
                *fan_out,
                stored(&fan_out_value),
                "fan-out is carried unchanged"
            );
            assert_eq!(*churn, stored(&churn_value), "churn is carried unchanged");
            match *change_impact {
                ChangeImpact::Analyses(ref rows) => {
                    assert_eq!(rows.as_slice(), &[stored(&impact_value)][..]);
                }
                ChangeImpact::NotApplicable(absence) => {
                    panic!("an injected change-impact was dropped as {absence:?}")
                }
            }
        }
        NormalizedStructuralGraph::Unavailable { ref reason, .. } => {
            panic!("an available graph was reported unavailable: {reason}")
        }
    }

    let rendered = render_governed_graph_portfolio(&report).expect("rendered text");
    assert!(
        rendered.contains("# QA portfolio report"),
        "the governed report is rendered by the ungoverned renderer it inherits"
    );
    assert!(
        rendered.contains("Fan-out: {\n  \"rows\""),
        "the opaque analysis is quoted, not summarised: {rendered}"
    );

    // A repository with no graph at all says so, and says why there is no
    // change-impact analysis rather than implying one.
    let absent = build(&[one("/repos/b", Vec::new())]);
    match absent.repositories[0].graph {
        NormalizedStructuralGraph::Unavailable { availability, .. } => {
            assert_eq!(availability, GapAvailability::Missing);
        }
        NormalizedStructuralGraph::Available { .. } => {
            panic!("a repository with no graph reported one")
        }
    }

    // Two of the three documents mapped is refused as incompatible, naming all
    // three.
    let partial = parse_graph_portfolio_mappings(
        &[PathBuf::from("/repos/a")],
        &GraphPortfolioMappingOptions {
            graph_exports: vec!["/repos/a=/inputs/export.json".to_owned()],
            cwd: Some(PathBuf::from("/repos")),
            ..GraphPortfolioMappingOptions::default()
        },
    )
    .expect("a partial mapping resolves to a refusal, not an error");
    match partial[0] {
        ResolvedMapping::Refused { status, .. } => {
            assert_eq!(status, MappingRefusal::Incompatible);
            assert!(
                partial[0]
                    .reason()
                    .is_some_and(|reason| reason.contains("export, premises, and audit")),
                "the refusal names every document the analysis needs"
            );
        }
        ResolvedMapping::Ready { .. } => panic!("a partial mapping was admitted"),
    }
    assert_eq!(
        ChangeImpactAbsence::ALL.len(),
        2,
        "both absences are stated sentences, not an implied one"
    );
}

/// Corrupt collections and graph inputs become local gaps that hide no sibling.
///
/// Trace: FR-067-AC-8
/// Provenance: quoin#480
#[test]
fn tc_480_044_corruption_is_a_local_gap() {
    let refused = GraphCollectionRead::refused(
        "/repos/a/broken.json".to_owned(),
        Some(CollectionUnavailability::Unreadable),
        Some("invalid JSON".to_owned()),
    );
    let report = build(&[
        repository(
            "/repos/a",
            vec![collection("good", "2026-08-31T00:00:00Z")],
            vec![refused],
            InjectedStructuralGraph::Unavailable {
                availability: GapAvailability::Unreadable,
                path: Some("/repos/a/graph.json".to_owned()),
                reason: "bad export".to_owned(),
            },
        ),
        one(
            "/repos/b",
            vec![collection("other", "2026-08-31T00:00:00Z")],
        ),
    ]);

    let first = &report.repositories[0];
    assert_eq!(
        first
            .graph_quality
            .current
            .as_ref()
            .map(|row| row.id.as_str()),
        Some("good"),
        "the readable sibling still produces a current reading"
    );
    assert!(
        first.gaps.iter().any(|gap| {
            gap.availability == GapAvailability::Unreadable
                && gap.path.as_deref() == Some("/repos/a/broken.json")
                && gap.owner.as_deref() == Some("assurance-team")
                && gap.action.as_deref() == Some("repair retained evidence")
        }),
        "the unreadable collection is a gap carrying the plan's governance, got {:?}",
        first.gaps
    );
    assert!(
        first.gaps.iter().any(|gap| {
            gap.availability == GapAvailability::Unreadable
                && gap.path.as_deref() == Some("/repos/a/graph.json")
        }),
        "the unreadable graph is a gap of its own"
    );
    assert_eq!(
        report.repositories[1]
            .graph_quality
            .current
            .as_ref()
            .map(|row| row.id.as_str()),
        Some("other"),
        "one repository's corruption does not hide another's evidence"
    );

    // Retained bytes that do not hash to the digest the producer declared are
    // not a current reading, and say so.
    let corrupt = build(&[one(
        "/repos/c",
        vec![collection_with(
            "bad-attachment",
            "2026-08-31T00:00:00Z",
            |value| {
                value["rawEvidence"]["scorer"]["bytesBase64"] =
                    Value::String("Y2hhbmdlZA==".to_owned());
            },
        )],
    )]);
    assert!(
        corrupt.repositories[0].graph_quality.current.is_none(),
        "a corrupt attachment is not a current reading"
    );
    assert!(
        corrupt.repositories[0].gaps.iter().any(|gap| {
            gap.availability == GapAvailability::Unreadable
                && gap
                    .path
                    .as_deref()
                    .is_some_and(|path| path.contains("bad-attachment"))
        }),
        "the corrupt attachment is a gap naming the record it is in"
    );
}

/// The report is canonical under permutation, and the mappings are too.
///
/// Trace: FR-067-AC-9
/// Provenance: quoin#480
#[test]
fn tc_480_045_the_report_and_the_mappings_are_canonical_under_permutation() {
    let build_from = |order: [(&str, [&str; 2]); 2]| {
        let inputs: Vec<GraphPortfolioRepositoryInput> = order
            .iter()
            .map(|(root, ids)| {
                one(
                    root,
                    ids.iter()
                        .map(|id| {
                            collection(
                                id,
                                if id.ends_with("old") {
                                    "2026-01-01T00:00:00Z"
                                } else {
                                    "2026-02-01T00:00:00Z"
                                },
                            )
                        })
                        .collect(),
                )
            })
            .collect();
        build(&inputs)
    };
    let expected = build_from([
        ("/repos/a", ["a-old", "a-new"]),
        ("/repos/b", ["b-old", "b-new"]),
    ]);
    let expected_json = canonical_graph_portfolio_json(&expected).expect("canonical JSON");
    let expected_text = render_governed_graph_portfolio(&expected).expect("rendered text");
    assert!(
        expected_text.contains("/repos/a") && expected_text.contains("History:"),
        "the rendered report consumes the report object"
    );

    for permutation in [
        [
            ("/repos/b", ["b-new", "b-old"]),
            ("/repos/a", ["a-new", "a-old"]),
        ],
        [
            ("/repos/a", ["a-new", "a-old"]),
            ("/repos/b", ["b-old", "b-new"]),
        ],
        [
            ("/repos/b", ["b-old", "b-new"]),
            ("/repos/a", ["a-old", "a-new"]),
        ],
    ] {
        let permuted = build_from(permutation);
        assert_eq!(
            canonical_graph_portfolio_json(&permuted).expect("canonical JSON"),
            expected_json,
            "{permutation:?}: the canonical JSON depends on input order"
        );
        assert_eq!(
            render_governed_graph_portfolio(&permuted).expect("rendered text"),
            expected_text,
            "{permutation:?}: the rendered report depends on input order"
        );
    }

    mappings_resolve_canonically();
}

/// Two spellings of one repository are one entry, its seeds are ordered and
/// de-duplicated, and one repository mapped twice to one kind of document is
/// refused by its own code.
fn mappings_resolve_canonically() {
    // Two spellings of one repository are one entry, and its seeds are ordered
    // and de-duplicated.
    let mappings = parse_graph_portfolio_mappings(
        &[PathBuf::from("/repos/a"), PathBuf::from("/repos/./a")],
        &GraphPortfolioMappingOptions {
            graph_exports: vec!["/repos/a=/inputs/export.json".to_owned()],
            graph_premises: vec!["/repos/./a=/inputs/premises.json".to_owned()],
            graph_audits: vec!["/repos/a=/inputs/audit.json".to_owned()],
            changed: vec![
                "/repos/a=FR-002".to_owned(),
                "/repos/./a=FR-001".to_owned(),
                "/repos/a=FR-001".to_owned(),
            ],
            cwd: Some(PathBuf::from("/repos")),
        },
    )
    .expect("the mappings resolve");
    assert_eq!(mappings.len(), 1, "two spellings are one repository");
    match mappings[0] {
        ResolvedMapping::Ready {
            ref root,
            ref export_path,
            ref premises_path,
            ref audit_path,
            ref changed,
        } => {
            assert_eq!(root, Path::new("/repos/a"));
            assert_eq!(export_path, Path::new("/inputs/export.json"));
            assert_eq!(premises_path, Path::new("/inputs/premises.json"));
            assert_eq!(audit_path, Path::new("/inputs/audit.json"));
            assert_eq!(changed, &["FR-001".to_owned(), "FR-002".to_owned()]);
        }
        ResolvedMapping::Refused { ref status, .. } => {
            panic!("a fully mapped repository was refused as {status:?}")
        }
    }

    for (field, code) in [
        (0_usize, GraphAdapterErrorCode::DuplicateGraphExport),
        (1, GraphAdapterErrorCode::DuplicateGraphPremises),
        (2, GraphAdapterErrorCode::DuplicateGraphAudit),
    ] {
        let pair = vec![
            "/repos/a=/inputs/one.json".to_owned(),
            "/repos/a=/inputs/two.json".to_owned(),
        ];
        let mut options = GraphPortfolioMappingOptions {
            cwd: Some(PathBuf::from("/repos")),
            ..GraphPortfolioMappingOptions::default()
        };
        match field {
            0 => options.graph_exports = pair,
            1 => options.graph_premises = pair,
            _ => options.graph_audits = pair,
        }
        let error = parse_graph_portfolio_mappings(&[PathBuf::from("/repos/a")], &options)
            .err()
            .unwrap_or_else(|| panic!("{code:?}: two documents of one kind were admitted"));
        assert_eq!(error.code(), code, "the duplicate is named by its own code");
    }
}

/// Historical evidence stays historical, and the report reaches no verdict of
/// its own.
///
/// Trace: FR-067-AC-10
/// Provenance: quoin#480
#[test]
fn tc_480_046_old_evidence_stays_historical_and_nothing_is_aggregated() {
    let report = build(&[one(
        "/repos/a",
        vec![collection("old", "2026-01-01T00:00:00Z")],
    )]);
    assert_eq!(
        report.repositories[0].graph_quality.history.len(),
        1,
        "the old collection is retained as history"
    );
    let canonical = canonical_graph_portfolio_json(&report).expect("canonical JSON");
    for forbidden in [
        "aggregate",
        "trustScore",
        "releaseVerdict",
        "qualityVerdict",
    ] {
        assert!(
            !canonical.contains(forbidden),
            "the report reached a verdict of its own: `{forbidden}`"
        );
    }
    assert!(
        !canonical.is_empty(),
        "an absence measured over an empty report measures nothing"
    );
}

/// The projection consumes injected objects: it executes nothing, traverses no
/// graph and writes nothing.
///
/// Trace: FR-067-AC-11
/// Provenance: quoin#480
#[test]
fn tc_480_047_the_projection_has_no_execution_traversal_or_write_dependency() {
    /// The modules that are `graph-portfolio.ts`. `loader` and `structural` are
    /// deliberately absent: they are `graph-portfolio-load.ts`, the one part
    /// that is *allowed* to read bytes and to call FR-062.
    const PROJECTION_MODULES: [&str; 14] = [
        "availability.rs",
        "build.rs",
        "compare.rs",
        "fields.rs",
        "history.rs",
        "input.rs",
        "mapping.rs",
        "order.rs",
        "reading.rs",
        "render.rs",
        "render_json.rs",
        "report.rs",
        "scorer.rs",
        "text.rs",
    ];

    /// What none of them may reach for.
    const FORBIDDEN: [&str; 8] = [
        "std::process",
        "std::net",
        "std::fs",
        "quoin_graph_analysis",
        "Command::new",
        "write_content_addressed",
        "adjacency",
        "traverse",
    ];

    /// Below this the census is reading an empty tree.
    const SOURCE_FLOOR: usize = 14;

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut measured = 0_usize;
    for module in PROJECTION_MODULES {
        let path = root.join(module);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: unreadable: {error}", path.display()));
        for needle in FORBIDDEN {
            assert!(
                !text.contains(needle),
                "{module}: the projection reaches for `{needle}`"
            );
        }
        measured += 1;
    }
    assert!(
        measured >= SOURCE_FLOOR,
        "the census read {measured} projection modules, below the floor of {SOURCE_FLOOR}"
    );

    // The two modules that *are* allowed to read exist, so the split above is a
    // split rather than a list that happens to name every module there is.
    for loader in ["loader.rs", "structural.rs"] {
        assert!(
            root.join(loader).is_file(),
            "{loader} is the boundary this census is drawn against"
        );
    }
}

/// The captured producer record, adapted by FR-066: the collection it becomes,
/// the plan it was governed by, and the two identities it declared.
fn adapted_producer_evidence() -> (MeasurementCollection, MeasurementPlan, String, String) {
    let capture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/graph-adapter-verdicts.json");
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(&capture)
            .unwrap_or_else(|error| panic!("{}: unreadable: {error}", capture.display())),
    )
    .expect("the capture is JSON");
    let input = golden["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["name"] == "quality/measured")
        .map(|case| case["input"].clone())
        .expect("the capture carries `quality/measured`");

    let text = |value: &str| -> quoin_measurement::types::ids::NonEmptyText {
        quoin_measurement::types::ids::NonEmptyText::parse(
            value,
            quoin_measurement::MeasurementErrorCode::PlanInvalid,
            "plan",
        )
        .expect("a non-empty plan member")
    };
    let producer_plan = MeasurementPlan {
        id: text(input["plans"][0]["id"].as_str().expect("a plan id")),
        title: text("Graph quality"),
        status: LifecycleStatus::Active,
        stage: MeasurementStage::Observe,
        metric: text("graph_quality"),
        definition_version: text(
            input["plans"][0]["definitionVersion"]
                .as_str()
                .expect("a definition version"),
        ),
        path: "spec/assurance/MP-001-graph-quality.md".to_owned(),
        owner: Some("assurance-team".to_owned()),
        action: Some("repair retained evidence".to_owned()),
        preregistration: None,
        ground_truth_kind: None,
        statistical_design: None,
    };
    let scorer_bytes: Vec<u8> = input["scorerBytes"]
        .as_array()
        .expect("scorer bytes")
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().expect("a byte")).expect("a byte"))
        .collect();
    let transcribed = quoin_measurement_graph::adapt_graph_quality_observation(
        &quoin_measurement_graph::quality::adapt::AdaptGraphQualityInput {
            record: &input["record"],
            scorer_bytes: &scorer_bytes,
            scorer_media_type: input["scorerMediaType"].as_str().expect("a media type"),
            attestation: &input["attestation"],
            plans: std::slice::from_ref(&producer_plan),
        },
    )
    .expect("the captured producer record is admissible");

    let declared_identity = input["record"]["observation_id"]
        .as_str()
        .expect("the record declares its identity")
        .to_owned();
    let declared_scorer = input["record"]["raw_scorer_output"]["digest"]
        .as_str()
        .expect("the record declares its scorer digest")
        .to_owned();
    (
        transcribed.collection().clone(),
        producer_plan,
        declared_identity,
        declared_scorer,
    )
}

/// Real producer evidence, adapted by FR-066 and then projected by FR-067 —
/// the two halves of StR-007 joined, as the deleted test joined them.
///
/// Trace: StR-007-VC-1
/// Provenance: quoin#480
#[test]
fn tc_480_048_adapted_evidence_reaches_the_portfolio_intact() {
    let (adapted, producer_plan, declared_identity, declared_scorer) = adapted_producer_evidence();
    let snapshot = PortfolioCollectionSnapshot {
        root: PathBuf::from("/repos/a"),
        collections: vec![adapted.clone()],
    };
    let base = build_portfolio_report_from_collections(&[snapshot])
        .repositories
        .into_iter()
        .next()
        .expect("the ungoverned portfolio carries the repository");
    let path = format!(
        "/repos/a/spec/evidence/measurements/{}.json",
        adapted.collection_id.as_str()
    );
    let report = build(&[GraphPortfolioRepositoryInput {
        portfolio: base,
        plans: vec![producer_plan],
        collections: vec![GraphCollectionRead::read(path, adapted)],
        graph: no_graph(),
    }]);

    let repository = &report.repositories[0];
    let current = repository
        .graph_quality
        .current
        .as_ref()
        .expect("the adapted evidence is a current reading");
    assert_eq!(
        current.producer_record_digest.as_deref(),
        Some(declared_identity.as_str()),
        "the producer's own record identity survives both halves unchanged"
    );
    assert_eq!(
        current.scorer_digest.as_deref(),
        Some(declared_scorer.as_str()),
        "the retained scorer identity survives both halves unchanged"
    );
    assert!(
        current.partitions.len() > 1,
        "the producer's partitions are not collapsed into one"
    );
    assert_eq!(
        repository.graph.availability(),
        GapAvailability::Missing.as_str(),
        "adapted quality evidence does not conjure a structural graph"
    );
    let canonical = canonical_graph_portfolio_json(&report).expect("canonical JSON");
    for forbidden in ["aggregateScore", "qualityVerdict", "child_process"] {
        assert!(
            !canonical.contains(forbidden),
            "the projection produced `{forbidden}`"
        );
    }
    let identities: BTreeSet<(&str, &str, &str)> = current
        .partitions
        .iter()
        .map(|row| {
            (
                row.identity.measure.as_str(),
                row.identity.dimension.as_str(),
                row.identity.key.as_str(),
            )
        })
        .collect();
    assert_eq!(
        identities.len(),
        current.partitions.len(),
        "each partition keeps its own identity"
    );
}
