// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Published measurement figures remain tied to their source records.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "invalid fixture evidence is a test failure"
)]

use std::path::PathBuf;

use ix_trace_rs::trace;
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::{from_serde, to_serde};
use quoin_measurement::portfolio::render::render_portfolio_report;
use quoin_measurement::portfolio::types::{
    PortfolioReport, PortfolioRepositoryReport, RepositoryStatus, Staleness, StoreState,
};
use quoin_measurement::report::build::{
    CurrentRow, MeasurementReport, build_measurement_report_from,
};
use quoin_measurement::report::render::render_measurement_report;
use quoin_measurement::types::ids::NonEmptyText;
use quoin_measurement::types::observation::{MeasurementShape, MeasurementState};
use quoin_measurement::types::plan::{LifecycleStatus, MeasurementPlan, MeasurementStage};
use quoin_measurement::{IntakeOrder, validate};
use serde_json::{Value, json};

struct Fixture {
    _root: tempfile::TempDir,
    repo: PathBuf,
    collection_path: PathBuf,
    artifact: Value,
    report: MeasurementReport,
    markdown: String,
    portfolio_markdown: String,
}

fn text(value: &str) -> NonEmptyText {
    NonEmptyText::parse(value, MeasurementErrorCode::PlanInvalid, "fixture field")
        .expect("fixture text is non-empty")
}

fn plan(id: &str, metric: &str, version: &str) -> MeasurementPlan {
    MeasurementPlan {
        id: text(id),
        title: text("Figure provenance fixture"),
        status: LifecycleStatus::Active,
        stage: MeasurementStage::Observe,
        metric: text(metric),
        definition_version: text(version),
        execution_procedure: None,
        path: format!("spec/assurance/{id}.md"),
        owner: None,
        action: None,
        ground_truth_kind: None,
        statistical_design: None,
        objective: None,
        protected_apparatus: None,
        negative_controls: None,
        cross_profile_comparison: false,
        paired_run: None,
    }
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().expect("fixture temp directory");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(&repo).expect("repo directory");
    let collection_id = "plat1965-figure-fixture";
    let artifact = json!({
        "schemaVersion": 2,
        "collectionId": collection_id,
        "subject": "fixture corpus",
        "scope": {"cases": 4},
        "toolIdentity": "fixture producer",
        "toolVersion": "1.0",
        "timestamp": "2026-11-27T00:00:00.000Z",
        "environment": {"runner": "test"},
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.measurement"],
            "artifacts": ["fixture.json"]
        },
        "observations": [
            {
                "metric": "quality.files",
                "planId": "MP-FILES",
                "definitionVersion": "quality.files-v1",
                "state": "measured",
                "value": 42,
                "unit": "files",
                "shape": "count",
                "population": {"examined": 42, "matched": 42, "complete": true, "identity": "source-tree"}
            },
            {
                "metric": "quality.coverage",
                "planId": "MP-COVERAGE",
                "definitionVersion": "quality.coverage-v1",
                "state": "measured",
                "value": 0.75,
                "unit": "fraction",
                "shape": "ratio",
                "population": {"examined": 4, "matched": 3, "complete": true, "identity": "fixture-population"}
            },
            {
                "metric": "finding_recall",
                "planId": "MP-RECALL",
                "definitionVersion": "finding.recall-v1",
                "state": "measured",
                "value": 0,
                "unit": "fraction",
                "shape": "ratio",
                "population": {"examined": 2, "matched": 0, "complete": true, "identity": "seeded-defects"}
            }
        ],
        "rawEvidence": {"fixture": true}
    });
    let collection_path = repo
        .join("spec/evidence/measurements")
        .join(format!("{collection_id}.json"));
    std::fs::create_dir_all(collection_path.parent().expect("collection parent"))
        .expect("measurement store directory");
    std::fs::write(
        &collection_path,
        serde_json::to_vec_pretty(&artifact).expect("artifact serializes"),
    )
    .expect("artifact writes");
    let stored = from_serde(&artifact).expect("artifact crosses the store bridge");
    let collection =
        validate::stored_measurement_collection(&stored).expect("fixture collection is valid");
    let plans = [
        plan("MP-FILES", "quality.files", "quality.files-v1"),
        plan("MP-COVERAGE", "quality.coverage", "quality.coverage-v1"),
        plan("MP-RECALL", "finding_recall", "finding.recall-v1"),
    ];
    let report =
        build_measurement_report_from(&repo, &plans, &[collection], &[], &[], &IntakeOrder::none())
            .expect("report builds from the source collection");
    let markdown = render_measurement_report(&report).expect("report renders");
    let portfolio = PortfolioReport {
        newest_collection_timestamp: Some("2026-11-27T00:00:00.000Z".to_owned()),
        repositories: vec![PortfolioRepositoryReport {
            name: "fixture".to_owned(),
            root: repo.to_string_lossy().into_owned(),
            status: RepositoryStatus::Readable,
            store: StoreState::Present,
            profiles: Vec::new(),
            plans: plans.to_vec(),
            measurements: Some(report.clone()),
            latest_collection: None,
            comparison: None,
            staleness: Staleness::NotComputed,
        }],
    };
    let portfolio_markdown = render_portfolio_report(&portfolio).expect("portfolio renders");
    Fixture {
        _root: root,
        repo,
        collection_path,
        artifact,
        report,
        markdown,
        portfolio_markdown,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FigureAudit {
    checked: usize,
    inconclusive: bool,
    violations: Vec<String>,
}

fn audit_figures(markdown: &str, rows: &[CurrentRow]) -> FigureAudit {
    let mut checked = 0usize;
    let mut violations = Vec::new();
    for row in rows {
        let Some(observation) = row.observation.as_ref() else {
            continue;
        };
        if observation.state != MeasurementState::Measured {
            continue;
        }
        let line = markdown
            .lines()
            .find(|line| line.starts_with(&format!("| {} |", row.metric)));
        let Some(line) = line else {
            violations.push(format!("{} has no rendered figure row", row.metric));
            continue;
        };
        checked += 1;
        let Some(comment) = line.split_once("<!-- ix-figure: ").map(|(_, tail)| tail) else {
            violations.push(format!("{} has no artifact/field binding", row.metric));
            continue;
        };
        let Some((encoded, _)) = comment.split_once(" -->") else {
            violations.push(format!("{} has a malformed figure binding", row.metric));
            continue;
        };
        let binding: Value = match serde_json::from_str(encoded) {
            Ok(binding) => binding,
            Err(error) => {
                violations.push(format!("{} has invalid binding JSON: {error}", row.metric));
                continue;
            }
        };
        let Some(collection) = row.collection.as_ref() else {
            violations.push(format!("{} has no source collection", row.metric));
            continue;
        };
        if binding["artifact"] != collection.path.as_str() {
            violations.push(format!("{} names the wrong source artifact", row.metric));
        }
        let Some(index) = row.observation_index else {
            violations.push(format!("{} has no observation index", row.metric));
            continue;
        };
        let expected_field = format!("/observations/{index}/value");
        if binding["field"] != expected_field {
            violations.push(format!("{} names the wrong source field", row.metric));
        }
        if line.contains(&format!("{}", observation.value.unwrap_or_default())) == false {
            violations.push(format!(
                "{} rendered value differs from its observation",
                row.metric
            ));
        }
        if binding["unit"] != observation.unit.as_str() {
            violations.push(format!("{} omits or changes its unit", row.metric));
        }
        if observation.shape == MeasurementShape::Ratio {
            let population = observation
                .population
                .as_ref()
                .and_then(|population| population.identity.as_ref());
            let population = population.map(|value| to_serde(value).expect("population is JSON"));
            if binding["population"] != population.unwrap_or(Value::Null) {
                violations.push(format!(
                    "{} omits or changes its population identifier",
                    row.metric
                ));
            }
        }
    }
    FigureAudit {
        checked,
        inconclusive: checked == 0,
        violations,
    }
}

fn audit_finding_recall_attention(markdown: &str, rows: &[CurrentRow]) -> FigureAudit {
    let mut checked = 0usize;
    let mut violations = Vec::new();
    for row in rows.iter().filter(|row| row.metric == "finding_recall") {
        let Some(observation) = row.observation.as_ref() else {
            continue;
        };
        if observation.state != MeasurementState::Measured || observation.value != Some(0.0) {
            continue;
        }
        let line = markdown
            .lines()
            .find(|line| line.contains("zero seeded defects detected"));
        let Some(line) = line else {
            violations.push("finding_recall attention figure is not rendered".to_owned());
            continue;
        };
        checked += 1;
        let Some(comment) = line.split_once("<!-- ix-figure: ").map(|(_, tail)| tail) else {
            violations
                .push("finding_recall attention figure has no artifact/field binding".to_owned());
            continue;
        };
        let Some((encoded, _)) = comment.split_once(" -->") else {
            violations.push("finding_recall attention binding is malformed".to_owned());
            continue;
        };
        let binding: Value = match serde_json::from_str(encoded) {
            Ok(binding) => binding,
            Err(error) => {
                violations.push(format!(
                    "finding_recall attention binding is invalid: {error}"
                ));
                continue;
            }
        };
        let Some(collection) = row.collection.as_ref() else {
            violations.push("finding_recall attention has no source collection".to_owned());
            continue;
        };
        if binding["artifact"] != collection.path.as_str() {
            violations.push("finding_recall attention names the wrong artifact".to_owned());
        }
        let Some(index) = row.observation_index else {
            violations.push("finding_recall attention has no observation index".to_owned());
            continue;
        };
        if binding["field"] != format!("/observations/{index}/value") {
            violations.push("finding_recall attention names the wrong field".to_owned());
        }
        if binding["unit"] != observation.unit.as_str() {
            violations.push("finding_recall attention omits or changes its unit".to_owned());
        }
        let population = observation
            .population
            .as_ref()
            .and_then(|population| population.identity.as_ref())
            .map(|value| to_serde(value).expect("population is JSON"));
        if binding["population"] != population.unwrap_or(Value::Null) {
            violations
                .push("finding_recall attention omits or changes population identity".to_owned());
        }
    }
    FigureAudit {
        checked,
        inconclusive: checked == 0,
        violations,
    }
}

#[trace("NFR-023-AC-1")]
#[test]
fn every_rendered_numeric_figure_names_its_collection_and_value_field() {
    let fixture = fixture();
    let audit = audit_figures(&fixture.markdown, &fixture.report.current);
    assert_eq!(audit.checked, 3, "all measured table figures are audited");
    assert!(audit.violations.is_empty(), "{:#?}", audit.violations);
    let attention_audit =
        audit_finding_recall_attention(&fixture.markdown, &fixture.report.current);
    assert_eq!(
        attention_audit.checked, 1,
        "finding recall attention figure is audited"
    );
    assert!(
        attention_audit.violations.is_empty(),
        "{:#?}",
        attention_audit.violations
    );
    let portfolio_audit = audit_figures(&fixture.portfolio_markdown, &fixture.report.current);
    assert_eq!(
        portfolio_audit.checked, 3,
        "portfolio figures are audited too"
    );
    assert!(
        portfolio_audit.violations.is_empty(),
        "{:#?}",
        portfolio_audit.violations
    );
}

#[trace("NFR-023-AC-2")]
#[test]
fn every_reported_value_recomputes_from_the_artifact_field_it_names() {
    let fixture = fixture();
    for row in &fixture.report.current {
        let Some(observation) = row.observation.as_ref() else {
            continue;
        };
        let index = row.observation_index.expect("source observation index");
        let recorded = fixture.artifact["observations"][index]["value"]
            .as_f64()
            .expect("artifact value is numeric");
        assert_eq!(observation.value, Some(recorded), "{}", row.metric);
        let line = fixture
            .markdown
            .lines()
            .find(|line| line.starts_with(&format!("| {} |", row.metric)))
            .expect("measured observation is rendered");
        assert!(
            line.contains(&format!("{}", recorded)),
            "{}: {line}",
            row.metric
        );
        assert!(
            row.collection.as_ref().is_some_and(|collection| {
                fixture.repo.join(&collection.path) == fixture.collection_path
            }),
            "{} is tied to the source artifact path",
            row.metric
        );
        let portfolio_line = fixture
            .portfolio_markdown
            .lines()
            .find(|line| line.starts_with(&format!("| {} |", row.metric)))
            .expect("portfolio observation is rendered");
        assert!(
            portfolio_line.contains("<!-- ix-figure: "),
            "{}: {portfolio_line}",
            row.metric
        );
    }
    let recall = fixture
        .report
        .current
        .iter()
        .find(|row| row.metric == "finding_recall")
        .expect("finding recall row exists");
    let attention_line = fixture
        .markdown
        .lines()
        .find(|line| line.contains("zero seeded defects detected"))
        .expect("finding recall attention figure is rendered");
    let encoded = attention_line
        .split_once("<!-- ix-figure: ")
        .and_then(|(_, tail)| tail.split_once(" -->"))
        .map(|(binding, _)| binding)
        .expect("finding recall attention binding is present");
    let binding: Value = serde_json::from_str(encoded).expect("attention binding is JSON");
    let index = recall.observation_index.expect("source observation index");
    assert_eq!(
        binding["artifact"],
        recall.collection.as_ref().expect("source collection").path
    );
    assert_eq!(binding["field"], format!("/observations/{index}/value"));
    assert_eq!(fixture.artifact["observations"][index]["value"], 0);
    assert!(attention_line.contains("zero seeded defects detected"));
}

#[trace("NFR-023-AC-3")]
#[test]
fn rendered_rates_carry_their_unit_and_population_identifier() {
    let fixture = fixture();
    let rate = fixture
        .report
        .current
        .iter()
        .find(|row| row.metric == "quality.coverage")
        .expect("rate row exists");
    let line = fixture
        .markdown
        .lines()
        .find(|line| line.starts_with("| quality.coverage |"))
        .expect("rate row is rendered");
    assert!(line.contains("fraction"), "unit is rendered: {line}");
    assert!(
        line.contains("fixture-population"),
        "population identifier is rendered: {line}"
    );
    let portfolio_line = fixture
        .portfolio_markdown
        .lines()
        .find(|line| line.starts_with("| quality.coverage |"))
        .expect("portfolio rate is rendered");
    assert!(
        portfolio_line.contains("fraction"),
        "unit is rendered: {portfolio_line}"
    );
    assert!(
        portfolio_line.contains("fixture-population"),
        "population identifier is rendered: {portfolio_line}"
    );
    assert_eq!(
        rate.observation
            .as_ref()
            .and_then(|observation| observation.population.as_ref())
            .and_then(|population| population.identity.as_ref())
            .map(|population| to_serde(population).expect("population identity is JSON")),
        Some(json!("fixture-population"))
    );
}

#[trace("NFR-023-AC-4")]
#[test]
fn figure_audit_reports_missing_bindings_and_incomplete_rates_and_marks_empty_as_inconclusive() {
    let fixture = fixture();
    let clean = audit_figures(&fixture.markdown, &fixture.report.current);
    assert_eq!(clean.checked, 3);
    assert!(clean.violations.is_empty(), "{:#?}", clean.violations);

    let numeric_line = fixture
        .markdown
        .lines()
        .find(|line| line.starts_with("| quality.files |"))
        .expect("numeric row");
    let missing_binding = fixture.markdown.replace(
        numeric_line,
        &numeric_line.replace("<!-- ix-figure: ", "<!-- removed: "),
    );
    let audit = audit_figures(&missing_binding, &fixture.report.current);
    assert!(
        audit
            .violations
            .iter()
            .any(|message| message.contains("no artifact/field binding"))
    );

    let attention_line = fixture
        .markdown
        .lines()
        .find(|line| line.contains("zero seeded defects detected"))
        .expect("finding recall attention figure");
    let missing_attention_binding = fixture.markdown.replace(
        attention_line,
        &attention_line.replace("<!-- ix-figure: ", "<!-- removed: "),
    );
    let attention_audit =
        audit_finding_recall_attention(&missing_attention_binding, &fixture.report.current);
    assert!(
        attention_audit
            .violations
            .iter()
            .any(|message| { message.contains("no artifact/field binding") })
    );

    let rate_line = fixture
        .markdown
        .lines()
        .find(|line| line.starts_with("| quality.coverage |"))
        .expect("rate row");
    let planted_rate = rate_line.replace("fixture-population", "");
    let missing_population = fixture.markdown.replace(rate_line, &planted_rate);
    let audit = audit_figures(&missing_population, &fixture.report.current);
    assert!(
        audit
            .violations
            .iter()
            .any(|message| message.contains("population identifier"))
    );

    let missing_unit = rate_line.replace("fraction", "");
    let missing_unit = fixture.markdown.replace(rate_line, &missing_unit);
    let audit = audit_figures(&missing_unit, &fixture.report.current);
    assert!(
        audit
            .violations
            .iter()
            .any(|message| message.contains("unit"))
    );

    let mut orphan = fixture.report.current[0].clone();
    orphan.collection = None;
    let audit = audit_figures(&fixture.markdown, &[orphan]);
    assert!(
        audit
            .violations
            .iter()
            .any(|message| message.contains("no source collection"))
    );

    let empty = audit_figures("# QA measurement report\n", &[]);
    assert_eq!(empty.checked, 0);
    assert!(
        empty.inconclusive,
        "an empty run is explicitly inconclusive"
    );
    assert!(empty.violations.is_empty());
}
