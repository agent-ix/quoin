// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The retained TypeScript's answers, captured once, replayed here.
//!
//! Trace: quoin#383-AC-1 — finding taxonomy, severities and thresholds
//! identical to the TypeScript on a golden corpus, exact.
//! Provenance: quoin#383
//!
//! The corpus is `tests/goldens/auditor.json`, captured by
//! `oracle/capture-auditor.mjs` and committed. **No test spawns node**
//! (FR-101-AC-5): the retained implementation is not a runtime oracle.
//!
//! Comparison is over **canonical JSON bytes**
//! ([`quoin_store::canonical_json`]), not over `serde_json::Value` equality.
//! That makes TypeScript's key insertion order a declared non-divergence
//! (`DIVERGENCE.md` §3) while leaving the comparison exact: a missing field, a
//! renamed field, a different number formatting or a different string all fail.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;

use quoin_auditor::advise::{ObligationEvidence, ObligationFacts};
use quoin_auditor::audit::AuditInput;
use quoin_auditor::catalog::{
    DiskModuleCatalogSource, MethodCatalog, ModuleRoot, load_method_catalog_from, method_classes,
};
use quoin_auditor::{
    MOCK_SUBJECT_FLOOR, advise, audit, characteristics_of, delta, mintable_characteristics,
    ratchet, scores_for, uncatalogued_authored_methods,
};
use quoin_evidence::types::{Binding, RunRecord};
use quoin_finding_types::AuditReport;
use quoin_quire_types::CoverageDiagnostic;
use serde::Deserialize;
use serde_json::Value;

// ── the corpus ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Golden {
    mock_subject_floor: f64,
    mintable_characteristics: Vec<String>,
    catalog_cases: Vec<CatalogCase>,
    audit_cases: Vec<AuditCase>,
    advise_catalog: MethodCatalog,
    advise_cases: Vec<AdviseCase>,
    characteristic_cases: Vec<CharacteristicCase>,
    ratchet_cases: Vec<RatchetCase>,
    delta: DeltaCase,
    uncatalogued_cases: Vec<UncataloguedCase>,
    scores_cases: Vec<ScoresCase>,
}

#[derive(Debug, Deserialize)]
struct CatalogCase {
    name: String,
    modules: BTreeMap<String, String>,
    roots: Vec<String>,
    catalog: Value,
    classes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AuditCase {
    name: String,
    input: AuditInput,
    report: Option<Value>,
    threw: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdviseCase {
    name: String,
    facts: ObligationFacts,
    advice: Value,
}

#[derive(Debug, Deserialize)]
struct CharacteristicCase {
    name: String,
    statement: String,
    criticality: Option<String>,
    evidence: Option<ObligationEvidence>,
    parameters: Option<BTreeMap<String, String>>,
    characteristics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RatchetCase {
    name: String,
    report: AuditReport,
    accepted: Vec<String>,
    remaining: Value,
}

#[derive(Debug, Deserialize)]
struct DeltaCase {
    before: AuditReport,
    after: AuditReport,
    added: Value,
    resolved: Value,
}

#[derive(Debug, Deserialize)]
struct UncataloguedCase {
    name: String,
    diagnostics: Vec<CoverageDiagnostic>,
    values: Vec<String>,
    degraded: bool,
}

#[derive(Debug, Deserialize)]
struct ScoresCase {
    name: String,
    bindings: Vec<Binding>,
    runs: Vec<RunRecord>,
    scores: Vec<f64>,
}

/// The committed corpus, deserialized once per test.
fn golden() -> Golden {
    let text = include_str!("goldens/auditor.json");
    serde_json::from_str(text).expect("the committed corpus parses")
}

/// Canonical JSON bytes, as [`quoin_store`] defines them and nothing else.
fn canonical(value: &Value) -> String {
    let bridged = quoin_measurement::json_bridge::from_serde(value)
        .expect("the corpus holds no value outside the JSON data model");
    quoin_store::canonical_json(&bridged).expect("canonicalization of parsed JSON cannot fail")
}

/// A Rust answer, canonicalized the same way.
fn canonical_of<T: serde::Serialize>(value: &T) -> String {
    canonical(&serde_json::to_value(value).expect("the answer serializes"))
}

// ── the audit ladder ───────────────────────────────────────────────────────

/// The audit ladder, replayed over the captured corpus. The criteria the
/// deleted `tests/auditor.test.ts` carried are restated here, each on the
/// golden case that decides it (quoin#501).
///
/// Trace: FR-032-AC-1, FR-032-AC-2, FR-032-AC-3, FR-032-AC-4, FR-032-AC-5
/// Trace: FR-032-AC-6, FR-032-AC-7, FR-032-AC-9, FR-032-AC-10, FR-032-AC-11
/// Trace: FR-032-AC-14, FR-032-AC-15, FR-032-AC-16, FR-032-CON-3
/// Trace: FR-039-AC-1, FR-039-AC-2, FR-039-AC-3, FR-039-AC-4, FR-039-AC-5
/// Trace: FR-039-AC-6, FR-039-AC-7, FR-039-AC-10, FR-039-AC-11, FR-039-AC-12
/// Trace: FR-039-CON-2, FR-039-CON-3
/// Trace: FR-035-AC-9, FR-035-AC-10, FR-035-AC-11, FR-035-CON-3
/// Provenance: quoin#383, quoin#501
#[test]
fn tc_383_001_every_audit_case_matches_the_retained_report() {
    let golden = golden();
    assert!(
        golden.audit_cases.len() >= 25,
        "anti-vacuity floor: the audit corpus must not shrink to nothing, saw {}",
        golden.audit_cases.len()
    );

    let mut kinds: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut compared = 0usize;
    let mut diverged = 0usize;
    for case in &golden.audit_cases {
        let produced = audit(&case.input)
            .unwrap_or_else(|cause| panic!("`{}` must not refuse: {cause}", case.name));
        for finding in &produced.findings {
            kinds.insert(finding.kind.clone());
        }
        let Some(expected) = case.report.as_ref() else {
            // DIVERGENCE §1: the retained code threw here. Asserted below, in
            // its own test, so that this loop never quietly skips a case.
            assert!(case.threw.is_some(), "`{}` has neither answer", case.name);
            diverged += 1;
            continue;
        };
        assert_eq!(
            canonical_of(&produced),
            canonical(expected),
            "`{}` must produce the retained report exactly",
            case.name
        );
        compared += 1;
    }

    assert!(
        compared >= 24,
        "anti-vacuity floor: at least 24 cases must be compared against a report, saw {compared}"
    );
    assert_eq!(
        diverged, 1,
        "exactly one case is a declared divergence; a second one is a regression, \
         not a corpus refresh"
    );
    assert!(
        kinds.len() >= 10,
        "anti-vacuity floor: the corpus must exercise at least 10 finding kinds, saw {kinds:?}"
    );
}

#[test]
fn tc_383_002_the_case_the_retained_code_throws_on_pairs_correctly_here() {
    // DIVERGENCE §1, with the must-have-fired assertion. `src/auditor/
    // audit.ts:415-419` filters `bindings` while indexing a `runs` array built
    // from `runBindings`; one scan-backed binding sorting ahead of a run-backed
    // one is enough to walk the index past the end.
    let golden = golden();
    let case = golden
        .audit_cases
        .iter()
        .find(|case| case.threw.is_some())
        .expect("the corpus must still carry the case the retained code throws on");
    let threw = case.threw.as_deref().unwrap_or_default();
    assert!(
        threw.contains("Cannot read properties of undefined"),
        "the retained failure must still be the undefined index, saw {threw:?}"
    );
    assert!(
        case.report.is_none(),
        "a case that threw has no retained report to compare against"
    );

    let produced = audit(&case.input).expect("the port must answer where the retained code threw");
    let stale: Vec<&str> = produced
        .findings
        .iter()
        .filter(|finding| finding.kind == "stale-evidence")
        .map(|finding| finding.summary.as_str())
        .collect();
    assert_eq!(
        stale.len(),
        1,
        "the run-backed binding is the one behind HEAD and must be reported once"
    );
    assert!(
        stale[0].contains("a unit run at"),
        "the finding must name the run-backed suite, not the scan-backed one: {stale:?}"
    );
}

// ── the advisor ────────────────────────────────────────────────────────────

/// The advice, replayed over the captured corpus.
///
/// Trace: FR-031-AC-2, FR-031-AC-3, FR-031-AC-4, FR-031-AC-5, FR-031-AC-6
/// Trace: FR-031-AC-7, FR-031-AC-8, FR-031-AC-18, FR-031-AC-19, FR-031-AC-22
/// Trace: FR-054-CON-2
/// Provenance: quoin#383, quoin#501
#[test]
fn tc_383_003_every_advise_case_matches_the_retained_advice() {
    let golden = golden();
    assert!(
        golden.advise_cases.len() >= 13,
        "anti-vacuity floor: the advice corpus must not shrink, saw {}",
        golden.advise_cases.len()
    );

    let mut recommended = 0usize;
    let mut mismatched = 0usize;
    let mut uncatalogued = 0usize;
    let mut multi_rule = 0usize;
    for case in &golden.advise_cases {
        let produced = advise(&golden.advise_catalog, &case.facts);
        assert_eq!(
            canonical_of(&produced),
            canonical(&case.advice),
            "`{}` must produce the retained advice exactly",
            case.name
        );
        if !produced.recommended.is_empty() {
            recommended += 1;
        }
        if produced.mismatch {
            mismatched += 1;
        }
        if produced.uncatalogued {
            uncatalogued += 1;
        }
        if produced
            .recommended
            .iter()
            .any(|entry| entry.reasons.len() >= 2)
        {
            multi_rule += 1;
        }
    }

    // A corpus of nothing-matched cases would pass the loop above while
    // proving that the advisor never recommends anything.
    assert!(
        recommended >= 5,
        "at least five cases must recommend something"
    );
    assert!(
        mismatched >= 1,
        "at least one case must be a genuine mismatch"
    );
    assert!(uncatalogued >= 1, "at least one case must be uncatalogued");
    assert!(
        multi_rule >= 1,
        "at least one case must match two rules at once"
    );
}

/// What a statement mints, replayed over the captured corpus.
///
/// Trace: FR-031-AC-14, FR-031-AC-15, FR-031-AC-17, FR-031-AC-19
/// Trace: FR-031-AC-20, FR-031-AC-21, FR-035-AC-12
/// Trace: FR-001-AC-9, FR-005-AC-1, FR-005-AC-2, FR-015-AC-3, FR-015-AC-4
/// Trace: FR-015-AC-6, NFR-006-AC-1, NFR-009-AC-1, NFR-022-M-12
/// Provenance: quoin#383, quoin#501
#[test]
fn tc_383_004_every_characteristic_case_matches() {
    let golden = golden();
    assert!(
        golden.characteristic_cases.len() >= 14,
        "anti-vacuity floor: the characteristic corpus must not shrink, saw {}",
        golden.characteristic_cases.len()
    );

    let mut matched = 0usize;
    let mut empty = 0usize;
    for case in &golden.characteristic_cases {
        let produced = characteristics_of(
            &case.statement,
            case.criticality.as_deref(),
            case.evidence.as_ref(),
            case.parameters.as_ref(),
        );
        assert_eq!(
            produced, case.characteristics,
            "`{}` must mint exactly what the retained table mints",
            case.name
        );
        if produced.is_empty() {
            empty += 1;
        } else {
            matched += 1;
        }
    }
    // Both halves matter: a table that matches nothing and a table that
    // matches everything each pass a one-sided floor.
    assert!(
        matched >= 8,
        "at least eight cases must mint a characteristic"
    );
    assert!(empty >= 4, "at least four cases must mint nothing");
}

#[test]
fn tc_383_005_the_mintable_set_is_the_retained_one() {
    let golden = golden();
    let produced: Vec<String> = mintable_characteristics().into_iter().collect();
    assert_eq!(
        produced, golden.mintable_characteristics,
        "the mintable set is derived from the regex table, so a table entry \
         lost in the port shows up here first"
    );
    assert!(
        produced.len() >= 50,
        "anti-vacuity floor: the table carries more than fifty characteristics, saw {}",
        produced.len()
    );
}

// ── the catalog reader ─────────────────────────────────────────────────────

/// The merged catalog, replayed over the captured corpus.
///
/// Trace: FR-031-AC-1, FR-031-AC-5, FR-031-AC-9, FR-032-AC-12
/// Provenance: quoin#383, quoin#501
#[test]
fn tc_383_006_every_catalog_case_matches_the_retained_merge() {
    let golden = golden();
    assert!(
        golden.catalog_cases.len() >= 9,
        "anti-vacuity floor: the catalog corpus must not shrink, saw {}",
        golden.catalog_cases.len()
    );

    let mut with_methods = 0usize;
    let mut with_duplicates = 0usize;
    let mut with_unreadable = 0usize;
    for case in &golden.catalog_cases {
        let home = tempfile::tempdir().expect("a temporary directory");
        let base = home.path();
        for (relative, text) in &case.modules {
            let dir = base.join(relative);
            std::fs::create_dir_all(&dir).expect("the module directory");
            std::fs::write(dir.join("manifest.yaml"), text).expect("the manifest");
        }
        let source = DiskModuleCatalogSource::new(base, None);
        let roots: Vec<ModuleRoot> = case
            .roots
            .iter()
            .map(|relative| {
                ModuleRoot::candidate(base.join(relative).to_string_lossy().into_owned())
            })
            .collect();
        let produced = load_method_catalog_from(&source, &roots);

        // The oracle rewrote its own temporary directory to `/modules`; this
        // rewrites ours, so the two answers are comparable at all.
        let rewritten = serde_json::to_string(&produced)
            .expect("the catalog serializes")
            .replace(&base.to_string_lossy().replace('\\', "\\\\"), "/modules");
        let mut produced_value: Value =
            serde_json::from_str(&rewritten).expect("the rewritten catalog parses");
        let mut expected = case.catalog.clone();
        // DIVERGENCE §2: the `reason` text is the reader's own message, and
        // Node's YAML parser and `quoin_yaml` do not write the same sentence.
        // Blanked on both sides here and asserted, non-empty and different, in
        // its own test below.
        blank_reasons(&mut produced_value);
        blank_reasons(&mut expected);
        assert_eq!(
            canonical(&produced_value),
            canonical(&expected),
            "`{}` must merge exactly as the retained reader does",
            case.name
        );
        assert_eq!(
            method_classes(&produced),
            case.classes,
            "`{}` must report the retained class set",
            case.name
        );

        if !produced.methods.is_empty() {
            with_methods += 1;
        }
        if !produced.duplicates.is_empty() {
            with_duplicates += 1;
        }
        if !produced.unreadable.is_empty() {
            with_unreadable += 1;
        }
    }

    assert!(with_methods >= 5, "at least five cases must merge a method");
    assert!(
        with_duplicates >= 2,
        "at least two cases must report a collision"
    );
    assert!(
        with_unreadable >= 1,
        "at least one case must report an unreadable module"
    );
}

/// Replace every `unreadable[].reason` with the empty string, in place.
fn blank_reasons(catalog: &mut Value) {
    if let Some(entries) = catalog
        .get_mut("unreadable")
        .and_then(serde_json::Value::as_array_mut)
    {
        for entry in entries {
            if let Some(reason) = entry.get_mut("reason") {
                *reason = Value::String(String::new());
            }
        }
    }
}

#[test]
fn tc_383_007_the_unreadable_reason_is_a_declared_divergence_that_fires() {
    // DIVERGENCE §2, with the must-have-fired assertion. Blanking a field in
    // the comparison above proves nothing unless the field was really there
    // and really different.
    let golden = golden();
    let case = golden
        .catalog_cases
        .iter()
        .find(|case| {
            case.catalog
                .get("unreadable")
                .and_then(Value::as_array)
                .is_some_and(|entries| !entries.is_empty())
        })
        .expect("the corpus must still carry an unreadable manifest");

    let retained = case.catalog["unreadable"][0]["reason"]
        .as_str()
        .expect("the retained reason is a string");
    assert!(
        !retained.is_empty(),
        "a blank retained reason would make the divergence unobservable"
    );

    let home = tempfile::tempdir().expect("a temporary directory");
    for (relative, text) in &case.modules {
        let dir = home.path().join(relative);
        std::fs::create_dir_all(&dir).expect("the module directory");
        std::fs::write(dir.join("manifest.yaml"), text).expect("the manifest");
    }
    let source = DiskModuleCatalogSource::new(home.path(), None);
    let roots: Vec<ModuleRoot> = case
        .roots
        .iter()
        .map(|relative| {
            ModuleRoot::candidate(home.path().join(relative).to_string_lossy().into_owned())
        })
        .collect();
    let produced = load_method_catalog_from(&source, &roots);

    assert_eq!(
        produced.unreadable.len(),
        case.catalog["unreadable"].as_array().map_or(0, Vec::len),
        "both readers must refuse the same modules — only the sentence differs"
    );
    let ours = &produced.unreadable[0].reason;
    assert!(
        !ours.is_empty(),
        "the port must say why it could not read the module"
    );
    assert_ne!(
        ours, retained,
        "if the two sentences ever agree this divergence is over and \
         DIVERGENCE.md §2 must be deleted rather than left standing"
    );
}

// ── the ratchet, the delta and the leaves ──────────────────────────────────

/// The ratchet, replayed over the captured corpus.
///
/// Trace: FR-032-AC-8
/// Provenance: quoin#383, quoin#501
#[test]
fn tc_383_008_every_ratchet_case_matches() {
    let golden = golden();
    assert_eq!(
        golden.ratchet_cases.len(),
        4,
        "anti-vacuity floor: the ratchet corpus is four cases"
    );
    let mut filtered = 0usize;
    for case in &golden.ratchet_cases {
        let produced = ratchet(&case.report, &case.accepted);
        assert_eq!(
            canonical_of(&produced),
            canonical(&case.remaining),
            "`{}` must leave exactly what the retained ratchet leaves",
            case.name
        );
        if produced.len() < case.report.findings.len() {
            filtered += 1;
        }
    }
    assert!(
        filtered >= 2,
        "a corpus where the baseline never filters anything would pass without \
         exercising the ratchet at all"
    );
}

#[test]
fn tc_383_009_the_delta_matches() {
    let golden = golden();
    let produced = delta(&golden.delta.before, &golden.delta.after);
    assert_eq!(
        canonical_of(&produced.added),
        canonical(&golden.delta.added)
    );
    assert_eq!(
        canonical_of(&produced.resolved),
        canonical(&golden.delta.resolved)
    );
    assert!(
        !produced.added.is_empty() && !produced.resolved.is_empty(),
        "the delta corpus must exercise both directions"
    );
}

/// The uncatalogued-method join, replayed over the captured corpus.
///
/// Trace: FR-031-AC-22, FR-031-AC-23
/// Provenance: quoin#383, quoin#501
#[test]
fn tc_383_010_the_uncatalogued_join_matches() {
    let golden = golden();
    assert!(golden.uncatalogued_cases.len() >= 4);
    let mut degraded = 0usize;
    let mut with_values = 0usize;
    for case in &golden.uncatalogued_cases {
        let produced = uncatalogued_authored_methods(&case.diagnostics);
        let values: Vec<String> = produced.values.iter().cloned().collect();
        assert_eq!(values, case.values, "`{}` values", case.name);
        assert_eq!(produced.degraded, case.degraded, "`{}` degraded", case.name);
        if produced.degraded {
            degraded += 1;
        }
        if !values.is_empty() {
            with_values += 1;
        }
    }
    assert!(
        degraded >= 1 && with_values >= 1,
        "both states must be exercised"
    );
}

/// The fault-detection score filter, replayed over the captured corpus.
///
/// Trace: FR-031-AC-18, FR-039-AC-10
/// Provenance: quoin#383, quoin#501
#[test]
fn tc_383_011_the_score_filter_matches() {
    let golden = golden();
    assert!(golden.scores_cases.len() >= 2);
    for case in &golden.scores_cases {
        let bindings: Vec<&Binding> = case.bindings.iter().collect();
        let runs: Vec<&RunRecord> = case.runs.iter().collect();
        assert_eq!(
            scores_for(&bindings, &runs),
            case.scores,
            "`{}` must select exactly the retained scores",
            case.name
        );
    }
    assert!(
        golden
            .scores_cases
            .iter()
            .any(|case| case.scores.len() == 1),
        "a corpus where every case selects nothing would pass without \
         exercising the filter"
    );
}

#[test]
fn tc_383_012_the_mock_subject_floor_is_the_retained_constant() {
    assert_eq!(MOCK_SUBJECT_FLOOR, golden().mock_subject_floor);
}
