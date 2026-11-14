// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Per-class evidence floors (FR-160), asserted on live `audit` calls over
//! inputs built here. The policy is data the test writes; nothing in quoin
//! supplies a class, a floor or a default (FR-160-AC-13).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quoin_auditor::audit::{AuditInput, audit};
use quoin_auditor::catalog::{MethodCatalog, VerificationMethod};
use quoin_auditor::{AuditorError, AuditorErrorCode};
use quoin_evidence::ids::{ObligationId, ProfileId, SuiteId, SymbolId};
use quoin_evidence::independence::assess_independence;
use quoin_evidence::types::{
    Binding, EvidenceLineage, FindingRecord, IndependenceDimension, IndependencePolicy,
    IndependenceRequirement, MUTATION_SCORE_METRIC, Outcome, RunEntry, RunRecord,
    STORE_SCHEMA_VERSION,
};
use quoin_finding_types::{AuditReport, Finding};
use quoin_quire_types::{CoverageMatrixCriterion, CoverageMatrixRequirement, Obligation};
use serde_json::{Value, json};

const OBLIGATION: &str = "FR-9-AC-1";

fn obligation(id: &str, criticality: Option<&str>) -> Obligation {
    Obligation {
        id: id.to_owned(),
        statement: "the system SHALL do the thing".to_owned(),
        target_ids: None,
        method: None,
        criticality: criticality.map(ToOwned::to_owned),
        parameters: None,
    }
}

fn lineage(actor: &str, technique: &str) -> EvidenceLineage {
    EvidenceLineage {
        actor: Some(actor.to_owned()),
        technique: Some(technique.to_owned()),
        ..EvidenceLineage::default()
    }
}

fn bind(obligation: &str, suite: &str, lineage: Option<EvidenceLineage>) -> Binding {
    Binding {
        obligation: ObligationId::new(obligation),
        suite: SuiteId::new(suite),
        symbols: vec![SymbolId::new(format!("{suite}::t"))],
        lineage,
    }
}

fn run(suite: &str, kind: &str, score: Option<f64>) -> RunRecord {
    let mut entry = RunEntry::new(format!("{suite}::t"), Outcome::Pass);
    if let Some(score) = score {
        entry.score = Some(score);
        entry.metric = Some(MUTATION_SCORE_METRIC.to_owned());
    }
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        tool: "some-tool".to_owned(),
        evidence_kind: Some(kind.to_owned()),
        timestamp: "2026-11-13T00:00:00Z".to_owned(),
        entries: vec![entry],
    }
}

fn method(id: &str, kind: &str) -> VerificationMethod {
    VerificationMethod {
        id: id.to_owned(),
        name: id.to_owned(),
        class: "Test".to_owned(),
        definition: String::new(),
        evidence_kind: Some(kind.to_owned()),
        applicability: BTreeMap::new(),
        tooling: Vec::new(),
        module_name: "m".to_owned(),
    }
}

/// The coverage entry for one obligation: the document states `carried`.
fn entry(obligation: &str, carried: &Value) -> CoverageMatrixRequirement {
    CoverageMatrixRequirement {
        document: format!("spec/{obligation}.md"),
        carried: carried.as_object().cloned(),
        criteria: vec![CoverageMatrixCriterion {
            id: obligation.to_owned(),
            method: None,
            binders: Vec::new(),
            identity: None,
            status: "tagged".to_owned(),
        }],
    }
}

/// An input whose one obligation states `class` under the carried name `class`.
/// Every suite is mock-inspected so `unevaluated` does not mask the subject.
fn scene(
    class: Option<&str>,
    policy: impl Into<Option<Value>>,
    bindings: Vec<Binding>,
    runs: Vec<RunRecord>,
) -> AuditInput {
    let suites = runs
        .iter()
        .map(|run| run.suite.as_str().to_owned())
        .collect();
    let carried = class.map_or_else(|| json!({}), |value| json!({ "class": value }));
    AuditInput {
        obligations: vec![obligation(OBLIGATION, None)],
        bindings,
        runs,
        mock_inspection_suites: Some(suites),
        catalog: Some(MethodCatalog {
            methods: vec![
                method("mutation-testing", "mutation-run"),
                method("fuzzing", "mutation-run"),
                method("unit-testing", "test-run"),
                method("blank-kind", ""),
                VerificationMethod {
                    evidence_kind: None,
                    ..method("no-kind", "")
                },
            ],
            duplicates: Vec::new(),
            unreadable: Vec::new(),
        }),
        class_policy: policy
            .into()
            .map(|policy| serde_json::from_value(policy).unwrap()),
        carried_vocabulary: Some(BTreeMap::from([(
            "class".to_owned(),
            vec![
                "class-1".to_owned(),
                "class-2".to_owned(),
                "class-3".to_owned(),
            ],
        )])),
        coverage_matrix: Some(vec![entry(OBLIGATION, &carried)]),
        ..AuditInput::default()
    }
}

fn policy(classes: &Value) -> Value {
    json!({ "class_field": "class", "classes": classes })
}

fn report(input: &AuditInput) -> AuditReport {
    audit(input).expect("a well-formed input is not refused")
}

fn of_kind<'a>(report: &'a AuditReport, kind: &str) -> Vec<&'a Finding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.kind == kind)
        .collect()
}

fn refused(input: &AuditInput) -> AuditorError {
    audit(input).expect_err("the policy is refused")
}

const RATIONALE: &str = "recorded by the safety lead";

// ── FR-160-AC-1 ───────────────────────────────────────────────────────────

/// Without a policy the audit reads no class and adds nothing, even when the
/// payload states one.
#[trace("FR-160-AC-1")]
#[test]
fn without_a_policy_the_report_carries_no_class_member_and_no_class_finding() {
    let mut input = scene(Some("class-2"), None, vec![], vec![]);
    input.class_policy = None;
    let out = report(&input);
    assert!(!out.other.contains_key("classReport"));
    assert!(!out.other.contains_key("classIndependence"));
    assert!(
        out.findings
            .iter()
            .all(|finding| !finding.kind.starts_with("class-"))
    );
    // The only finding is the ladder's own, as it was before this feature.
    assert_eq!(of_kind(&out, "undischarged").len(), 1);
}

// ── FR-160-AC-2 ───────────────────────────────────────────────────────────

/// A required method is satisfied by evidence of its catalog kind, whichever
/// method of that kind produced it, and missing otherwise.
#[trace("FR-160-AC-2")]
#[test]
fn a_required_method_needs_bound_evidence_of_its_catalog_kind() {
    let classes =
        json!({ "class-2": { "required_methods": ["mutation-testing"], "rationale": RATIONALE } });
    // Unit-test-kind evidence only: the mutation kind is missing.
    let missing = report(&scene(
        Some("class-2"),
        policy(&classes),
        vec![bind(OBLIGATION, "unit", None)],
        vec![run("unit", "test-run", None)],
    ));
    let found = of_kind(&missing, "class-method-missing");
    assert_eq!(found.len(), 1, "{:?}", missing.findings);
    for named in ["class-2", "mutation-testing", "mutation-run", OBLIGATION] {
        assert!(found[0].summary.contains(named), "{}", found[0].summary);
    }

    // Evidence of the same kind from another method's suite clears it.
    let cleared = report(&scene(
        Some("class-2"),
        policy(&classes),
        vec![bind(OBLIGATION, "fuzz", None)],
        vec![run("fuzz", "mutation-run", None)],
    ));
    assert!(of_kind(&cleared, "class-method-missing").is_empty());
}

/// An obligation with no evidence at all is missing the class's methods too.
#[trace("FR-160-AC-2")]
#[test]
fn an_unbound_obligation_is_missing_its_class_methods_beside_undischarged() {
    let classes =
        json!({ "class-2": { "required_methods": ["unit-testing"], "rationale": RATIONALE } });
    let out = report(&scene(Some("class-2"), policy(&classes), vec![], vec![]));
    assert_eq!(of_kind(&out, "undischarged").len(), 1);
    assert_eq!(of_kind(&out, "class-method-missing").len(), 1);
}

// ── FR-160-AC-3 ───────────────────────────────────────────────────────────

/// The class mutation floor is evaluated by the FR-039 rule and names the class.
#[trace("FR-160-AC-3")]
#[test]
fn a_class_mutation_floor_yields_insufficient_and_unmeasured_findings() {
    let classes = json!({ "class-2": { "mutation_floor": 0.8, "rationale": RATIONALE } });
    let bound = || vec![bind(OBLIGATION, "mut", None)];

    let low = report(&scene(
        Some("class-2"),
        policy(&classes),
        bound(),
        vec![run("mut", "mutation-run", Some(0.6))],
    ));
    let found = of_kind(&low, "insufficient-mutation-score");
    assert_eq!(found.len(), 1, "{:?}", low.findings);
    assert!(
        found[0].summary.contains("class class-2"),
        "{}",
        found[0].summary
    );
    assert!(found[0].summary.contains("0.8"), "{}", found[0].summary);

    let unmeasured = report(&scene(
        Some("class-2"),
        policy(&classes),
        bound(),
        vec![run("mut", "mutation-run", None)],
    ));
    let found = of_kind(&unmeasured, "unmeasured-mutation-score");
    assert_eq!(found.len(), 1, "{:?}", unmeasured.findings);
    assert!(
        found[0].summary.contains("class class-2"),
        "{}",
        found[0].summary
    );

    let enough = report(&scene(
        Some("class-2"),
        policy(&classes),
        bound(),
        vec![run("mut", "mutation-run", Some(0.8))],
    ));
    assert!(enough.findings.is_empty(), "{:?}", enough.findings);
}

/// A class without a floor is unaffected by another class's floor.
#[trace("FR-160-AC-3")]
#[test]
fn another_classs_mutation_floor_does_not_apply() {
    let classes = json!({
        "class-1": { "rationale": RATIONALE },
        "class-2": { "mutation_floor": 0.8, "rationale": RATIONALE }
    });
    let out = report(&scene(
        Some("class-1"),
        policy(&classes),
        vec![bind(OBLIGATION, "mut", None)],
        vec![run("mut", "mutation-run", Some(0.1))],
    ));
    assert!(out.findings.is_empty(), "{:?}", out.findings);
}

// ── FR-160-AC-4 ───────────────────────────────────────────────────────────

fn class_3() -> Value {
    policy(&json!({
        "class-3": { "independence": ["actor", "technique"], "rationale": RATIONALE }
    }))
}

fn two_suites(first: EvidenceLineage, second: EvidenceLineage) -> AuditInput {
    scene(
        Some("class-3"),
        class_3(),
        vec![
            bind(OBLIGATION, "a", Some(first)),
            bind(OBLIGATION, "b", Some(second)),
        ],
        vec![run("a", "test-run", None), run("b", "test-run", None)],
    )
}

/// Two lines sharing an actor fail; two differing on every dimension pass; one
/// line alone fails.
#[trace("FR-160-AC-4")]
#[test]
fn class_independence_needs_two_lines_differing_on_every_listed_dimension() {
    let shared = report(&two_suites(
        lineage("ann", "fuzz"),
        lineage("ann", "review"),
    ));
    let found = of_kind(&shared, "insufficient-independence");
    assert_eq!(found.len(), 1, "{:?}", shared.findings);
    assert!(found[0].summary.contains("class-3"), "{}", found[0].summary);
    // Only the dimension the two lines truly share is named.
    assert!(
        found[0]
            .summary
            .ends_with("Still shared or missing: actor (shared)."),
        "{}",
        found[0].summary
    );
    assert!(!found[0].summary.contains("technique (shared)"));
    assert_eq!(found[0].other.get("source"), Some(&json!("class")));

    let apart = report(&two_suites(
        lineage("ann", "fuzz"),
        lineage("bob", "review"),
    ));
    assert!(
        of_kind(&apart, "insufficient-independence").is_empty(),
        "{:?}",
        apart.findings
    );

    let alone = report(&scene(
        Some("class-3"),
        class_3(),
        vec![bind(OBLIGATION, "a", Some(lineage("ann", "fuzz")))],
        vec![run("a", "test-run", None)],
    ));
    let found = of_kind(&alone, "insufficient-independence");
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .summary
            .ends_with("Fewer than two evidence lines are bound."),
        "{}",
        found[0].summary
    );
}

/// A class independence record exists for each obligation of the class, even
/// one the ladder stops before the independence check (here: nothing bound).
#[trace("FR-160-AC-4")]
#[test]
fn an_undischarged_obligation_of_the_class_still_has_its_assessment_record() {
    let out = report(&scene(Some("class-3"), class_3(), vec![], vec![]));
    let records = &out.other["classIndependence"];
    assert_eq!(records.as_array().map(Vec::len), Some(1), "{records}");
    assert_eq!(records[0]["status"], "insufficient");
    assert_eq!(records[0]["obligation"], OBLIGATION);
}

/// Under a class policy the profile records say `source: profile`; without
/// one the report's `independence` member is what it always was.
#[trace("FR-160-AC-10")]
#[test]
fn profile_assessment_records_carry_their_source_only_under_a_class_policy() {
    let under = profile_selecting_actor(lineage("ann", "fuzz"), lineage("bob", "fuzz"));
    let out = report(&under);
    assert_eq!(out.other["independence"][0]["source"], "profile");
    let mut without = under;
    without.class_policy = None;
    let out = report(&without);
    assert!(out.other["independence"][0].get("source").is_none());
}

/// The assessment record states its source, class, requirement and dimensions.
#[trace("FR-160-AC-4")]
#[test]
fn the_class_assessment_record_names_source_class_requirement_and_dimensions() {
    let out = report(&two_suites(
        lineage("ann", "fuzz"),
        lineage("bob", "review"),
    ));
    let records = out
        .other
        .get("classIndependence")
        .expect("recorded under a policy");
    let record = &records[0];
    assert_eq!(record["source"], "class");
    assert_eq!(record["class"], "class-3");
    assert_eq!(record["requirement"], "FR-9");
    assert_eq!(record["obligation"], OBLIGATION);
    assert_eq!(record["status"], "satisfied");
    assert_eq!(record["dimensions"].as_array().map(Vec::len), Some(2));
}

// ── FR-160-AC-5 ───────────────────────────────────────────────────────────

fn crit_and_class(
    criticality: Option<&str>,
    class: Option<&str>,
    class_floor: f64,
    crit_floor: f64,
) -> AuditReport {
    let classes = json!({ "class-2": { "mutation_floor": class_floor, "rationale": RATIONALE } });
    let mut input = scene(
        class,
        policy(&classes),
        vec![bind(OBLIGATION, "mut", None)],
        vec![run("mut", "mutation-run", Some(0.5))],
    );
    input.obligations = vec![obligation(OBLIGATION, criticality)];
    input.mutation_floor = Some(BTreeMap::from([("high".to_owned(), crit_floor)]));
    report(&input)
}

/// Criticality and class are separate axes, each selecting its own floors.
#[trace("FR-160-AC-5")]
#[test]
fn criticality_and_class_floors_are_separate_axes_and_the_higher_ratio_wins() {
    let by_criticality = crit_and_class(Some("high"), None, 0.9, 0.7);
    let found = of_kind(&by_criticality, "insufficient-mutation-score");
    assert_eq!(found.len(), 1);
    assert!(
        found[0].summary.contains("criticality high"),
        "{}",
        found[0].summary
    );

    let by_class = crit_and_class(None, Some("class-2"), 0.9, 0.7);
    let found = of_kind(&by_class, "insufficient-mutation-score");
    assert_eq!(found.len(), 1);
    assert!(
        found[0].summary.contains("class class-2"),
        "{}",
        found[0].summary
    );

    let class_higher = crit_and_class(Some("high"), Some("class-2"), 0.9, 0.7);
    let found = of_kind(&class_higher, "insufficient-mutation-score");
    assert_eq!(found.len(), 1);
    assert!(
        found[0].summary.contains("class class-2"),
        "{}",
        found[0].summary
    );
    assert!(found[0].summary.contains("0.9"), "{}", found[0].summary);

    let tie = crit_and_class(Some("high"), Some("class-2"), 0.7, 0.7);
    let found = of_kind(&tie, "insufficient-mutation-score");
    assert_eq!(found.len(), 1);
    assert!(
        found[0].summary.contains("criticality high"),
        "a tie names criticality: {}",
        found[0].summary
    );

    let crit_higher = crit_and_class(Some("high"), Some("class-2"), 0.6, 0.7);
    let found = of_kind(&crit_higher, "insufficient-mutation-score");
    assert_eq!(found.len(), 1);
    assert!(
        found[0].summary.contains("criticality high"),
        "{}",
        found[0].summary
    );
    assert!(found[0].summary.contains("0.7"), "{}", found[0].summary);
}

// ── FR-160-AC-6 ───────────────────────────────────────────────────────────

/// An obligation whose requirement states no class gets no class floor and is
/// reported as unassigned.
#[trace("FR-160-AC-6")]
#[test]
fn an_unclassified_obligation_gets_no_class_floor_and_is_reported_unassigned() {
    let classes = json!({ "class-2": { "required_methods": ["unit-testing"], "mutation_floor": 0.9, "rationale": RATIONALE } });
    let out = report(&scene(
        None,
        policy(&classes),
        vec![bind(OBLIGATION, "mut", None)],
        vec![run("mut", "mutation-run", Some(0.1))],
    ));
    assert!(out.findings.is_empty(), "{:?}", out.findings);
    let class_report = out
        .other
        .get("classReport")
        .expect("present under a policy");
    assert_eq!(class_report["unassigned"], json!([OBLIGATION]));
    assert_eq!(class_report["classes"], json!({}));
}

// ── FR-160-AC-7 ───────────────────────────────────────────────────────────

/// A class with no policy entry yields one `class-unconfigured` finding and no
/// other class finding.
#[trace("FR-160-AC-7")]
#[test]
fn a_class_without_a_policy_entry_is_unconfigured_and_nothing_else() {
    let classes =
        json!({ "class-2": { "required_methods": ["unit-testing"], "rationale": RATIONALE } });
    let out = report(&scene(
        Some("class-3"),
        policy(&classes),
        vec![bind(OBLIGATION, "unit", None)],
        vec![run("unit", "test-run", None)],
    ));
    let unconfigured = of_kind(&out, "class-unconfigured");
    assert_eq!(unconfigured.len(), 1, "{:?}", out.findings);
    assert!(unconfigured[0].summary.contains("class-3"));
    assert!(
        out.findings
            .iter()
            .all(|finding| finding.kind == "class-unconfigured"),
        "{:?}",
        out.findings
    );
}

// ── FR-160-AC-8 ───────────────────────────────────────────────────────────

fn refusal_of(classes: &Value) -> String {
    let error = refused(&scene(Some("class-2"), policy(classes), vec![], vec![]));
    assert_eq!(error.code, AuditorErrorCode::ClassPolicyRefused);
    error.message.to_string()
}

/// Each way a policy cannot be applied is refused, naming the cause.
#[trace("FR-160-AC-8")]
#[test]
fn a_policy_that_cannot_be_applied_is_refused_naming_the_cause() {
    let cases = [
        (json!({ "class-9": { "rationale": RATIONALE } }), "class-9"),
        (
            json!({ "class-2": { "required_methods": ["telepathy"], "rationale": RATIONALE } }),
            "telepathy",
        ),
        (
            json!({ "class-2": { "mutation_floor": 80.0, "rationale": RATIONALE } }),
            "[0, 1]",
        ),
        (
            json!({ "class-2": { "independence": ["vibes"], "rationale": RATIONALE } }),
            "vibes",
        ),
        (json!({ "class-2": { "rationale": "  " } }), "rationale"),
        (
            json!({ "class-2": { "required_methods": ["no-kind"], "rationale": RATIONALE } }),
            "no-kind",
        ),
        (
            json!({ "class-2": { "required_methods": ["blank-kind"], "rationale": RATIONALE } }),
            "blank-kind",
        ),
    ];
    for (classes, cause) in cases {
        let message = refusal_of(&classes);
        assert!(message.contains(cause), "{cause} not named in {message}");
    }
}

/// With no vocabulary for the class field nothing says which classes exist.
#[trace("FR-160-AC-8")]
#[test]
fn a_report_without_the_vocabulary_for_the_class_field_refuses_the_policy() {
    let mut input = scene(
        Some("class-2"),
        policy(&json!({ "class-2": { "rationale": RATIONALE } })),
        vec![],
        vec![],
    );
    input.carried_vocabulary = None;
    let error = refused(&input);
    assert_eq!(error.code, AuditorErrorCode::ClassPolicyRefused);
    assert!(error.message.contains("carried_vocabulary"), "{error}");
}

// ── FR-160-AC-9 ───────────────────────────────────────────────────────────

/// The class is the requirement's own: another carried field naming a part's
/// class changes nothing.
#[trace("FR-160-AC-9", "FR-160-CON-1")]
#[test]
fn the_class_is_read_only_from_the_declared_carried_name() {
    let classes = json!({
        "class-1": { "mutation_floor": 0.1, "rationale": RATIONALE },
        "class-3": { "mutation_floor": 0.99, "rationale": RATIONALE }
    });
    let mut input = scene(
        Some("class-1"),
        policy(&classes),
        vec![bind(OBLIGATION, "mut", None)],
        vec![run("mut", "mutation-run", Some(0.5))],
    );
    input.coverage_matrix = Some(vec![entry(
        OBLIGATION,
        &json!({ "class": "class-1", "item_class": "class-3" }),
    )]);
    let out = report(&input);
    assert!(out.findings.is_empty(), "{:?}", out.findings);
}

/// A criticality is never read as a class.
#[trace("FR-160-CON-1")]
#[test]
fn criticality_is_never_read_as_a_class() {
    let classes =
        json!({ "class-1": { "required_methods": ["unit-testing"], "rationale": RATIONALE } });
    let mut input = scene(None, policy(&classes), vec![], vec![]);
    input.obligations = vec![obligation(OBLIGATION, Some("class-1"))];
    let out = report(&input);
    assert!(
        of_kind(&out, "class-method-missing").is_empty(),
        "{:?}",
        out.findings
    );
}

// ── FR-160-AC-10 ──────────────────────────────────────────────────────────

fn profile_selecting_actor(first: EvidenceLineage, second: EvidenceLineage) -> AuditInput {
    let mut input = two_suites(first, second);
    let requirement = IndependenceRequirement {
        id: "IND-1".to_owned(),
        obligation: ObligationId::new(OBLIGATION),
        dimensions: vec![IndependenceDimension::Actor],
        rationale: "profile".to_owned(),
    };
    let policy = IndependencePolicy {
        schema_version: 1,
        profile: ProfileId::parse("AP-1").unwrap(),
        requirements: vec![requirement.clone()],
    };
    input.independence = Some(vec![assess_independence(
        &policy.profile,
        &requirement,
        &input.bindings,
    )]);
    input.independence_policy = Some(policy);
    input
}

/// Profile and class independence each hold on their own dimensions; each
/// failure is its own finding naming its source.
#[trace("FR-160-AC-10")]
#[test]
fn profile_and_class_independence_are_assessed_separately_and_both_must_hold() {
    let sources = |out: &AuditReport| -> Vec<String> {
        let mut sources: Vec<String> = of_kind(out, "insufficient-independence")
            .iter()
            .map(|finding| finding.other["source"].as_str().unwrap().to_owned())
            .collect();
        sources.sort();
        sources
    };
    // Differ on technique, share an actor: the profile (actor) fails.
    let profile_only = report(&profile_selecting_actor(
        lineage("ann", "fuzz"),
        lineage("ann", "review"),
    ));
    // The class lists actor too, so both fail here; use a class on technique
    // alone for the isolated cases below.
    assert_eq!(sources(&profile_only), ["class", "profile"]);

    let mut technique_only =
        profile_selecting_actor(lineage("ann", "fuzz"), lineage("ann", "review"));
    technique_only.class_policy = serde_json::from_value(policy(
        &json!({ "class-3": { "independence": ["technique"], "rationale": RATIONALE } }),
    ))
    .unwrap();
    assert_eq!(sources(&report(&technique_only)), ["profile"]);

    let mut class_only = profile_selecting_actor(lineage("ann", "fuzz"), lineage("bob", "fuzz"));
    class_only.class_policy = technique_only.class_policy.clone();
    let out = report(&class_only);
    assert_eq!(sources(&out), ["class"]);
    assert!(
        of_kind(&out, "insufficient-independence")[0]
            .summary
            .contains("class-3")
    );
}

// ── FR-160-AC-11 ──────────────────────────────────────────────────────────

/// The class column holds the value as written; an absent class has no entry.
#[trace("FR-160-AC-11")]
#[test]
fn the_class_report_carries_the_value_as_written_and_omits_the_absent() {
    let classes = json!({ "class-2": { "rationale": RATIONALE } });
    let mut input = scene(Some("class-2"), policy(&classes), vec![], vec![]);
    input.obligations.push(obligation("FR-9-AC-2", None));
    let out = report(&input);
    let class_report = &out.other["classReport"];
    assert_eq!(class_report["classes"], json!({ OBLIGATION: "class-2" }));
    assert_eq!(class_report["unassigned"], json!(["FR-9-AC-2"]));
}

// ── FR-160-AC-12 ──────────────────────────────────────────────────────────

/// The same inputs give the same report, byte for byte.
#[trace("FR-160-AC-12", "NFR-034")]
#[test]
fn two_audits_of_the_same_input_are_identical() {
    let classes = json!({
        "class-3": { "required_methods": ["unit-testing", "mutation-testing"], "independence": ["actor"], "mutation_floor": 0.9, "rationale": RATIONALE }
    });
    let input = scene(
        Some("class-3"),
        policy(&classes),
        vec![bind(OBLIGATION, "a", Some(lineage("ann", "x")))],
        vec![run("a", "other-kind", Some(0.2))],
    );
    let first = serde_json::to_string(&report(&input)).unwrap();
    let second = serde_json::to_string(&report(&input)).unwrap();
    assert_eq!(first, second);
    assert!(first.contains("class-method-missing"));
}

/// A misspelt key is refused at the file boundary, not silently ignored.
#[trace("FR-160-AC-8")]
#[test]
fn an_unknown_policy_key_is_refused() {
    let misspelt = json!({
        "class_field": "class",
        "classes": { "class-2": { "mutation_flor": 0.8, "rationale": RATIONALE } }
    });
    let error = serde_json::from_value::<quoin_auditor::audit::ClassPolicy>(misspelt)
        .expect_err("an unknown key is refused");
    assert!(error.to_string().contains("mutation_flor"), "{error}");
}

/// The obligations key belongs to FR-161: its entries are not validated here.
#[trace("FR-160-AC-8")]
#[test]
fn obligations_entries_are_accepted_unread() {
    let classes = json!({ "class-2": { "obligations": [{ "id": "x", "kind": "design-trace" }], "rationale": RATIONALE } });
    let out = report(&scene(Some("class-2"), policy(&classes), vec![], vec![]));
    assert!(of_kind(&out, "class-unconfigured").is_empty());
}

/// A scanner-only suite satisfies a required method of its evidence kind.
#[trace("FR-160-AC-2")]
#[test]
fn a_bound_scan_record_of_the_kind_satisfies_a_required_method() {
    let classes =
        json!({ "class-2": { "required_methods": ["mutation-testing"], "rationale": RATIONALE } });
    let mut input = scene(
        Some("class-2"),
        policy(&classes),
        vec![bind(OBLIGATION, "scanner", None)],
        vec![],
    );
    let scan = |kind: &str| FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("scanner"),
        tool: "some-scanner".to_owned(),
        evidence_kind: Some(kind.to_owned()),
        timestamp: "2026-11-13T00:00:00Z".to_owned(),
        ruleset: None,
        rules_evaluated: Some(3),
        findings: Vec::new(),
        evaluated: None,
    };
    input.scans = Some(vec![scan("mutation-run")]);
    assert!(of_kind(&report(&input), "class-method-missing").is_empty());
    input.scans = Some(vec![scan("test-run")]);
    assert_eq!(of_kind(&report(&input), "class-method-missing").len(), 1);
}
