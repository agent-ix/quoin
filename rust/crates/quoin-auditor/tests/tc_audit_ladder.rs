// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The audit ladder (FR-032), the mutation-score floor (FR-039) and the
//! combinatorial finding (FR-035), each asserted on live `audit` calls over
//! inputs built in the test. Nothing is replayed from a recorded report.

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
use quoin_auditor::catalog::{
    MemoryModuleCatalogSource, MethodCatalog, ModuleRoot, VerificationMethod,
    load_method_catalog_from,
};
use quoin_evidence::ids::{ObligationId, SuiteId, SymbolId};
use quoin_evidence::types::{
    Binding, MUTATION_SCORE_METRIC, MockInjection, Outcome, RunEntry, RunRecord,
    STORE_SCHEMA_VERSION,
};
use quoin_finding_types::{AuditReport, Finding, Severity};
use quoin_quire_types::Obligation;

fn ob(id: &str, statement: &str) -> Obligation {
    Obligation {
        id: id.to_owned(),
        statement: statement.to_owned(),
        target_ids: None,
        method: None,
        criticality: None,
        parameters: None,
    }
}

fn ob_with(id: &str, method: Option<&str>, criticality: Option<&str>) -> Obligation {
    Obligation {
        method: method.map(ToOwned::to_owned),
        criticality: criticality.map(ToOwned::to_owned),
        ..ob(id, "the system SHALL do the thing")
    }
}

fn bind(obligation: &str, suite: &str, symbols: &[&str]) -> Binding {
    Binding {
        obligation: ObligationId::new(obligation),
        suite: SuiteId::new(suite),
        symbols: symbols
            .iter()
            .map(|symbol| SymbolId::new(*symbol))
            .collect(),
        lineage: None,
    }
}

fn run_of(suite: &str, kind: Option<&str>, entries: Vec<RunEntry>) -> RunRecord {
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        tool: "some-tool".to_owned(),
        evidence_kind: kind.map(ToOwned::to_owned),
        timestamp: "2026-09-27T00:00:00Z".to_owned(),
        entries,
    }
}

fn pass(symbol: &str) -> RunEntry {
    RunEntry::new(symbol, Outcome::Pass)
}

fn scored(symbol: &str, outcome: Outcome, score: f64, metric: Option<&str>) -> RunEntry {
    RunEntry {
        score: Some(score),
        metric: metric.map(ToOwned::to_owned),
        ..RunEntry::new(symbol, outcome)
    }
}

/// An input with every named suite already inspected for mocks,
/// so the `unevaluated` bucket does not mask what a test is about.
fn input(obligations: Vec<Obligation>, bindings: Vec<Binding>, runs: Vec<RunRecord>) -> AuditInput {
    let suites = runs
        .iter()
        .map(|run| run.suite.as_str().to_owned())
        .collect();
    AuditInput {
        obligations,
        bindings,
        runs,
        mock_inspection_suites: Some(suites),
        ..AuditInput::default()
    }
}

fn report(input: &AuditInput) -> AuditReport {
    audit(input).expect("a well-formed input is never refused")
}

fn of_kind<'a>(report: &'a AuditReport, kind: &str, obligation: &str) -> Vec<&'a Finding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.kind == kind && finding.obligation == obligation)
        .collect()
}

fn method(id: &str, class: &str, evidence_kind: Option<&str>) -> VerificationMethod {
    VerificationMethod {
        id: id.to_owned(),
        name: id.to_owned(),
        class: class.to_owned(),
        definition: String::new(),
        evidence_kind: evidence_kind.map(ToOwned::to_owned),
        applicability: BTreeMap::new(),
        tooling: Vec::new(),
        module_name: "m".to_owned(),
    }
}

fn catalog() -> MethodCatalog {
    MethodCatalog {
        methods: vec![
            method("unit-testing", "Test", Some("test-run")),
            method("model-checking", "Analysis", Some("analysis-report")),
        ],
        duplicates: Vec::new(),
        unreadable: Vec::new(),
    }
}

// ── FR-032 ────────────────────────────────────────────────────────────────

/// A passing run produces no finding and is healthy.
#[trace("FR-032-AC-1")]
#[test]
fn a_passing_run_at_head_is_healthy_and_silent() {
    let audited = input(
        vec![ob("FR-9-AC-1", "the system SHALL do the thing")],
        vec![bind("FR-9-AC-1", "unit", &["t::a"])],
        vec![run_of("unit", None, vec![pass("t::a")])],
    );
    let out = report(&audited);
    assert!(out.findings.is_empty(), "{:?}", out.findings);
    assert_eq!(out.healthy, ["FR-9-AC-1"]);
}

/// A missing recorded run is high severity.
#[trace("FR-032-AC-3")]
#[test]
fn a_missing_run_is_high() {
    let audited = input(
        vec![ob("FR-9-AC-1", "the system SHALL do the thing")],
        vec![bind("FR-9-AC-1", "ghost", &["t::a"])],
        vec![],
    );
    let out = report(&audited);
    let missing = of_kind(&out, "stale-evidence", "FR-9-AC-1");
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].severity, Some(Severity::high()));
}

/// Vacuity fires when every bound symbol was skipped or absent, and does not
/// fire when at least one passed.
#[trace("FR-032-AC-4")]
#[test]
fn vacuity_needs_every_bound_symbol_skipped_or_absent() {
    let audited = input(
        vec![
            ob("FR-9-AC-1", "the system SHALL do the thing"),
            ob("FR-9-AC-2", "the system SHALL do another thing"),
        ],
        vec![
            bind("FR-9-AC-1", "unit", &["t::skipped", "t::absent"]),
            bind("FR-9-AC-2", "unit", &["t::skipped", "t::ok"]),
        ],
        vec![run_of(
            "unit",
            None,
            vec![RunEntry::new("t::skipped", Outcome::Skip), pass("t::ok")],
        )],
    );
    let out = report(&audited);
    let vacuous = of_kind(&out, "vacuous-evidence", "FR-9-AC-1");
    assert_eq!(vacuous.len(), 1);
    assert_eq!(vacuous[0].severity, Some(Severity::high()));
    assert_eq!(of_kind(&out, "vacuous-evidence", "FR-9-AC-2").len(), 0);
}

/// An obligation with no binding is undischarged at medium severity.
#[trace("FR-032-AC-5")]
#[test]
fn an_unbound_obligation_is_undischarged_at_medium() {
    let out = report(&input(
        vec![ob("FR-9-AC-1", "the system SHALL do")],
        vec![],
        vec![],
    ));
    let found = of_kind(&out, "undischarged", "FR-9-AC-1");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Some(Severity::medium()));
    assert_eq!(out.healthy.len(), 0);
}

/// A non-test-class method discharged by a test run is flagged; a test-class
/// one is not; and with no catalog the question is not asked.
#[trace("FR-032-AC-6")]
#[test]
fn a_non_test_method_discharged_by_a_test_run_is_flagged() {
    let build = |declared: &str, catalog: Option<MethodCatalog>| {
        let mut audited = input(
            vec![ob_with("FR-9-AC-1", Some(declared), None)],
            vec![bind("FR-9-AC-1", "unit", &["t::a"])],
            vec![run_of("unit", Some("test-run"), vec![pass("t::a")])],
        );
        audited.catalog = catalog;
        report(&audited)
    };
    let flagged = build("model-checking", Some(catalog()));
    assert_eq!(
        of_kind(&flagged, "method-conformance", "FR-9-AC-1").len(),
        1
    );
    let test_class = build("Test", Some(catalog()));
    assert_eq!(
        of_kind(&test_class, "method-conformance", "FR-9-AC-1").len(),
        0
    );
    let no_catalog = build("model-checking", None);
    assert!(no_catalog.findings.is_empty(), "{:?}", no_catalog.findings);
}

/// A criticality demanding two independent methods is satisfied by two suites
/// and not by two symbols in one, and does not apply below the threshold.
#[trace("FR-032-AC-7")]
#[test]
fn two_independent_methods_means_two_suites_not_two_symbols() {
    let build = |criticality: &str, bindings: Vec<Binding>| {
        let mut audited = input(
            vec![ob_with("FR-9-AC-1", None, Some(criticality))],
            bindings,
            vec![
                run_of("unit", None, vec![pass("t::a"), pass("t::b")]),
                run_of("prop", None, vec![pass("t::c")]),
            ],
        );
        audited.multiplicity_requires = Some(vec!["P0".to_owned()]);
        report(&audited)
    };
    let one_suite = build("P0", vec![bind("FR-9-AC-1", "unit", &["t::a", "t::b"])]);
    assert_eq!(
        of_kind(&one_suite, "insufficient-multiplicity", "FR-9-AC-1").len(),
        1
    );
    let two_suites = build(
        "P0",
        vec![
            bind("FR-9-AC-1", "unit", &["t::a"]),
            bind("FR-9-AC-1", "prop", &["t::c"]),
        ],
    );
    assert_eq!(
        of_kind(&two_suites, "insufficient-multiplicity", "FR-9-AC-1").len(),
        0
    );
    let below = build("P1", vec![bind("FR-9-AC-1", "unit", &["t::a"])]);
    assert_eq!(
        of_kind(&below, "insufficient-multiplicity", "FR-9-AC-1").len(),
        0
    );
}

/// A run declaring no evidence kind is not judged: the question cannot be
/// asked, which is different from the answer being yes.
#[trace("FR-032-AC-9")]
#[test]
fn a_run_declaring_no_kind_is_not_judged_for_conformance() {
    let build = |kind: Option<&str>| {
        let mut audited = input(
            vec![ob_with("FR-9-AC-1", Some("model-checking"), None)],
            vec![bind("FR-9-AC-1", "unit", &["t::a"])],
            vec![run_of("unit", kind, vec![pass("t::a")])],
        );
        audited.catalog = Some(catalog());
        report(&audited)
    };
    assert_eq!(build(None).findings.len(), 0);
    assert_eq!(build(Some("")).findings.len(), 0);
    assert_eq!(
        of_kind(&build(Some("test-run")), "method-conformance", "FR-9-AC-1").len(),
        1,
        "the same run, once it declares a mismatching kind, is judged"
    );
    let matching = build(Some("analysis-report"));
    assert_eq!(
        of_kind(&matching, "method-conformance", "FR-9-AC-1").len(),
        0
    );
}

/// A declared method matching neither a catalog id nor a class is reported.
#[trace("FR-032-AC-10")]
#[test]
fn an_undeclared_method_is_reported_as_unknown() {
    let mut audited = input(
        vec![
            ob_with("FR-9-AC-1", Some("Smoke (TC-1)"), None),
            ob_with("FR-9-AC-2", Some("unit-testing"), None),
            ob_with("FR-9-AC-3", Some("analysis"), None),
        ],
        vec![],
        vec![],
    );
    audited.catalog = Some(catalog());
    let out = report(&audited);
    assert_eq!(of_kind(&out, "unknown-method", "FR-9-AC-1").len(), 1);
    assert_eq!(of_kind(&out, "unknown-method", "FR-9-AC-2").len(), 0);
    assert_eq!(of_kind(&out, "unknown-method", "FR-9-AC-3").len(), 0);
}

/// The catalog is merged from exactly the module roots supplied: an ambient
/// module cannot extend an explicit set, and the supplied order decides which
/// module wins a colliding id.
#[trace("FR-032-AC-12")]
#[test]
fn an_explicit_module_set_supplies_the_catalog_and_ambient_modules_cannot_extend_it() {
    let manifest = |name: &str, id: &str| {
        format!(
            "name: {name}\nverification_catalog:\n  {id}:\n    name: {id}\n    class: Test\n  shared:\n    name: from-{name}\n    class: Test\n"
        )
    };
    let source = MemoryModuleCatalogSource::new()
        .with_module("/ambient", manifest("ambient", "ambient-only"))
        .with_module("/first", manifest("first", "first-only"))
        .with_module("/second", manifest("second", "second-only"));
    let explicit = [
        ModuleRoot::candidate("/second"),
        ModuleRoot::candidate("/first"),
    ];
    let merged = load_method_catalog_from(&source, &explicit);
    let ids: Vec<&str> = merged.methods.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["first-only", "second-only", "shared"]);
    let shared = merged.methods.iter().find(|m| m.id == "shared").unwrap();
    assert_eq!(
        shared.module_name, "second",
        "the first supplied module wins"
    );
    assert_eq!(merged.duplicates[0].modules, ["second", "first"]);
}

/// `unknown-method` is evaluated before the binding guard: an unbound
/// obligation with an uncatalogued method is both undischarged and unknown.
#[trace("FR-032-AC-14")]
#[test]
fn unknown_method_fires_whatever_the_evidence_state() {
    let mut audited = input(
        vec![ob_with("FR-9-AC-1", Some("Smoke"), None)],
        vec![],
        vec![],
    );
    audited.catalog = Some(catalog());
    let out = report(&audited);
    assert_eq!(of_kind(&out, "undischarged", "FR-9-AC-1").len(), 1);
    assert_eq!(of_kind(&out, "unknown-method", "FR-9-AC-1").len(), 1);
}

fn injection(suite: &str, symbol: &str, injects: &[&str]) -> MockInjection {
    MockInjection {
        suite: SuiteId::new(suite),
        symbol: SymbolId::new(symbol),
        injects: injects.iter().map(|name| (*name).to_owned()).collect(),
        path: Some("tests/parser.rs".to_owned()),
        line: Some(42),
    }
}

/// An obligation discharged only by tests injecting a stand-in for its own
/// subject is `mocked-confirmation` at medium, with the locus; one real suite
/// alongside a mocked one is ordinary test design.
#[trace("FR-032-AC-15")]
#[test]
fn only_mocked_bindings_make_a_mocked_confirmation() {
    let statement = "the parser SHALL reject malformed input";
    let build = |bindings: Vec<Binding>, injections: Vec<MockInjection>| {
        let mut audited = input(
            vec![ob("FR-9-AC-1", statement)],
            bindings,
            vec![
                run_of("unit", None, vec![pass("t::mocked")]),
                run_of("real", None, vec![pass("t::real")]),
            ],
        );
        audited.injections = Some(injections);
        report(&audited)
    };
    let stand_in = injection("unit", "t::mocked", &["fake_parser_reject"]);

    let only_mocked = build(
        vec![bind("FR-9-AC-1", "unit", &["t::mocked"])],
        vec![stand_in.clone()],
    );
    let found = of_kind(&only_mocked, "mocked-confirmation", "FR-9-AC-1");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Some(Severity::medium()));
    assert_eq!(found[0].path.as_deref(), Some("tests/parser.rs"));
    assert_eq!(found[0].line, Some(42));
    assert_eq!(found[0].symbol.as_deref(), Some("t::mocked"));

    let mixed = build(
        vec![
            bind("FR-9-AC-1", "unit", &["t::mocked"]),
            bind("FR-9-AC-1", "real", &["t::real"]),
        ],
        vec![stand_in],
    );
    assert_eq!(of_kind(&mixed, "mocked-confirmation", "FR-9-AC-1").len(), 0);

    let unrelated = build(
        vec![bind("FR-9-AC-1", "unit", &["t::mocked"])],
        vec![injection("unit", "t::mocked", &["clock", "tempdir"])],
    );
    assert_eq!(
        of_kind(&unrelated, "mocked-confirmation", "FR-9-AC-1").len(),
        0
    );
}

/// The auditor skips a check it cannot perform when the catalog is absent.
#[trace("FR-032-CON-3")]
#[test]
fn an_absent_catalog_removes_the_method_check() {
    let mut audited = input(
        vec![ob_with("FR-9-AC-1", Some("Smoke"), None)],
        vec![bind("FR-9-AC-1", "unit", &["t::a"])],
        vec![run_of("unit", None, vec![pass("t::a")])],
    );
    audited.catalog = None;
    let out = report(&audited);
    assert!(out.findings.is_empty(), "{:?}", out.findings);
    assert_eq!(out.healthy, ["FR-9-AC-1"]);
}

// ── FR-039 ────────────────────────────────────────────────────────────────

fn floor(entries: &[(&str, f64)]) -> BTreeMap<String, f64> {
    entries
        .iter()
        .map(|(criticality, value)| ((*criticality).to_owned(), *value))
        .collect()
}

/// One P0 obligation bound to one suite whose run holds the given entries.
fn mutation_audit(
    entries: Vec<RunEntry>,
    floors: Option<BTreeMap<String, f64>>,
    criticality: &str,
) -> AuditReport {
    let symbols: Vec<String> = entries
        .iter()
        .map(|entry| entry.symbol.as_str().to_owned())
        .collect();
    let symbol_refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
    let mut audited = input(
        vec![ob_with("FR-9-AC-1", None, Some(criticality))],
        vec![bind("FR-9-AC-1", "mutants", &symbol_refs)],
        vec![run_of("mutants", None, entries)],
    );
    audited.mutation_floor = floors;
    report(&audited)
}

fn mutation_kinds(out: &AuditReport) -> Vec<&str> {
    out.findings
        .iter()
        .filter(|finding| finding.kind.ends_with("mutation-score"))
        .map(|finding| finding.kind.as_str())
        .collect()
}

/// With no floor declared, no mutation finding is produced whatever the score.
#[trace("FR-039-AC-1", "FR-039-CON-2")]
#[test]
fn no_declared_floor_produces_no_mutation_finding() {
    let out = mutation_audit(
        vec![scored(
            "t::a",
            Outcome::Pass,
            0.0,
            Some(MUTATION_SCORE_METRIC),
        )],
        None,
        "P0",
    );
    assert_eq!(mutation_kinds(&out).len(), 0);
    assert_eq!(out.healthy, ["FR-9-AC-1"]);
}

/// A bound score below the floor is reported at medium.
#[trace("FR-039-AC-2")]
#[test]
fn a_score_below_the_floor_is_reported_at_medium() {
    let out = mutation_audit(
        vec![scored(
            "t::a",
            Outcome::Pass,
            0.5,
            Some(MUTATION_SCORE_METRIC),
        )],
        Some(floor(&[("P0", 0.8)])),
        "P0",
    );
    let found = of_kind(&out, "insufficient-mutation-score", "FR-9-AC-1");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Some(Severity::medium()));
}

/// A score exactly at the floor is accepted.
#[trace("FR-039-AC-3")]
#[test]
fn a_score_exactly_at_the_floor_is_accepted() {
    let out = mutation_audit(
        vec![scored(
            "t::a",
            Outcome::Pass,
            0.8,
            Some(MUTATION_SCORE_METRIC),
        )],
        Some(floor(&[("P0", 0.8)])),
        "P0",
    );
    assert_eq!(mutation_kinds(&out).len(), 0);
}

/// The judgement is on the weakest bound symbol, not the mean.
#[trace("FR-039-AC-4")]
#[test]
fn the_weakest_symbol_decides_not_the_mean() {
    let out = mutation_audit(
        vec![
            scored("t::strong", Outcome::Pass, 1.0, Some(MUTATION_SCORE_METRIC)),
            scored("t::weak", Outcome::Pass, 0.7, Some(MUTATION_SCORE_METRIC)),
        ],
        Some(floor(&[("P0", 0.8)])),
        "P0",
    );
    let found = of_kind(&out, "insufficient-mutation-score", "FR-9-AC-1");
    assert_eq!(
        found.len(),
        1,
        "the mean 0.85 clears the floor; the worst 0.7 does not"
    );
    assert!(found[0].summary.contains("0.7"), "{}", found[0].summary);
}

/// A demanded floor with nothing measured is its own finding, distinct from
/// `undischarged`; a passing run or coverage never stands in for a score.
#[trace("FR-039-AC-5", "FR-039-CON-3")]
#[test]
fn a_demanded_floor_with_nothing_measured_is_unmeasured_not_undischarged() {
    let out = mutation_audit(vec![pass("t::a")], Some(floor(&[("P0", 0.8)])), "P0");
    assert_eq!(
        of_kind(&out, "unmeasured-mutation-score", "FR-9-AC-1").len(),
        1
    );
    assert_eq!(of_kind(&out, "undischarged", "FR-9-AC-1").len(), 0);
    assert!(!out.healthy.contains(&"FR-9-AC-1".to_owned()));
}

/// A skipped symbol's absent score is not read as zero.
#[trace("FR-039-AC-6")]
#[test]
fn a_skipped_symbols_score_is_not_read_as_zero() {
    let out = mutation_audit(
        vec![
            scored(
                "t::skipped",
                Outcome::Skip,
                0.0,
                Some(MUTATION_SCORE_METRIC),
            ),
            scored("t::ran", Outcome::Pass, 0.9, Some(MUTATION_SCORE_METRIC)),
        ],
        Some(floor(&[("P0", 0.8)])),
        "P0",
    );
    assert!(mutation_kinds(&out).is_empty(), "{:?}", out.findings);
}

/// A floor applies only to the criticality it names.
#[trace("FR-039-AC-7")]
#[test]
fn a_floor_applies_only_to_the_criticality_it_names() {
    let weak = || {
        vec![scored(
            "t::a",
            Outcome::Pass,
            0.1,
            Some(MUTATION_SCORE_METRIC),
        )]
    };
    let other = mutation_audit(weak(), Some(floor(&[("P0", 0.8)])), "P1");
    assert_eq!(mutation_kinds(&other).len(), 0);
    let named = mutation_audit(weak(), Some(floor(&[("P0", 0.8)])), "P0");
    assert_eq!(mutation_kinds(&named), ["insufficient-mutation-score"]);
}

/// A score whose entry declares another metric, or none, is not compared
/// against the floor.
#[trace("FR-039-AC-10")]
#[test]
fn a_score_labelled_with_another_metric_is_not_compared() {
    let out = mutation_audit(
        vec![
            scored(
                "t::mutants",
                Outcome::Pass,
                0.9,
                Some(MUTATION_SCORE_METRIC),
            ),
            scored("t::latency", Outcome::Pass, 0.1, Some("latency-ms")),
            scored("t::unlabelled", Outcome::Pass, 0.2, None),
        ],
        Some(floor(&[("P0", 0.8)])),
        "P0",
    );
    assert!(mutation_kinds(&out).is_empty(), "{:?}", out.findings);
}

/// The floor filters on the entry's metric, never on the run's tool name: a
/// labelled score from a tool no catalog lists is judged like any other.
#[trace("FR-039-AC-11")]
#[test]
fn the_metric_and_not_the_tool_decides_whether_a_score_is_judged() {
    let mut audited = input(
        vec![ob_with("FR-9-AC-1", None, Some("P0"))],
        vec![bind("FR-9-AC-1", "mutants", &["t::a"])],
        vec![RunRecord {
            tool: "a-mutation-tool-nobody-lists".to_owned(),
            ..run_of(
                "mutants",
                None,
                vec![scored(
                    "t::a",
                    Outcome::Pass,
                    0.5,
                    Some(MUTATION_SCORE_METRIC),
                )],
            )
        }],
    );
    audited.mutation_floor = Some(floor(&[("P0", 0.8)]));
    let out = report(&audited);
    assert_eq!(mutation_kinds(&out), ["insufficient-mutation-score"]);
}

/// The check reads no catalog, and an entry without a metric is not a mutation
/// score even when the run's tool is a mutation tool.
#[trace("FR-039-AC-12")]
#[test]
fn an_unlabelled_entry_is_not_a_score_whatever_the_tool_says() {
    let mut audited = input(
        vec![ob_with("FR-9-AC-1", None, Some("P0"))],
        vec![bind("FR-9-AC-1", "mutants", &["t::a"])],
        vec![RunRecord {
            tool: "cargo-mutants".to_owned(),
            ..run_of(
                "mutants",
                None,
                vec![scored("t::a", Outcome::Pass, 1.0, None)],
            )
        }],
    );
    audited.mutation_floor = Some(floor(&[("P0", 0.8)]));
    assert!(audited.catalog.is_none());
    let out = report(&audited);
    assert_eq!(mutation_kinds(&out), ["unmeasured-mutation-score"]);
}

// ── FR-035 ────────────────────────────────────────────────────────────────

fn configured(symbol: &str, pairs: &[(&str, &str)]) -> RunEntry {
    RunEntry {
        config: Some(
            pairs
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        ),
        ..pass(symbol)
    }
}

fn space_audit(statement: &str, entries: Vec<RunEntry>) -> AuditReport {
    let symbols: Vec<String> = entries
        .iter()
        .map(|entry| entry.symbol.as_str().to_owned())
        .collect();
    let refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
    report(&input(
        vec![ob("FR-9-AC-1", statement)],
        vec![bind("FR-9-AC-1", "matrix", &refs)],
        vec![run_of("matrix", None, entries)],
    ))
}

const SPACE: &str = "2-way over a(1|2) b(3|4)";

/// `audit` reports `combinatorial-gap` naming the missing combinations, and the
/// finding is `medium`.
#[trace("FR-035-AC-9", "FR-035-CON-3")]
#[test]
fn an_incomplete_covering_array_is_a_medium_gap_naming_the_missing_combinations() {
    let out = space_audit(
        SPACE,
        vec![
            configured("t::one", &[("a", "1"), ("b", "3")]),
            configured("t::two", &[("a", "2"), ("b", "4")]),
        ],
    );
    let found = of_kind(&out, "combinatorial-gap", "FR-9-AC-1");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Some(Severity::medium()));
    assert!(
        found[0].summary.contains("a=1 & b=4"),
        "{}",
        found[0].summary
    );
    assert!(
        found[0].summary.contains("a=2 & b=3"),
        "{}",
        found[0].summary
    );
    assert!(
        !found[0].summary.contains("a=1 & b=3"),
        "{}",
        found[0].summary
    );
}

/// An obligation whose demanded combinations all ran is healthy.
#[trace("FR-035-AC-10")]
#[test]
fn an_obligation_whose_combinations_all_ran_is_healthy() {
    let out = space_audit(
        SPACE,
        vec![
            configured("t::1", &[("a", "1"), ("b", "3")]),
            configured("t::2", &[("a", "1"), ("b", "4")]),
            configured("t::3", &[("a", "2"), ("b", "3")]),
            configured("t::4", &[("a", "2"), ("b", "4")]),
        ],
    );
    assert!(out.findings.is_empty(), "{:?}", out.findings);
    assert_eq!(out.healthy, ["FR-9-AC-1"]);
}

/// An obligation declaring no configuration space produces no combinatorial
/// finding.
#[trace("FR-035-AC-11")]
#[test]
fn an_obligation_with_no_declared_space_produces_no_combinatorial_finding() {
    let out = space_audit("the system SHALL do the thing", vec![pass("t::a")]);
    assert!(
        out.findings
            .iter()
            .all(|finding| finding.kind != "combinatorial-gap")
    );
    assert_eq!(out.healthy, ["FR-9-AC-1"]);
}
