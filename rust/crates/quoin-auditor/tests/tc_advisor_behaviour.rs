// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The verification-method advisor (FR-031) and the structural combinatorial
//! signal (FR-035-AC-12), asserted on live `advise` / `characteristics_of`
//! calls over statements and catalogs written in the test. Nothing is replayed
//! from a recorded advice document.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use quoin_auditor::advise::{PropertyShape, UncataloguedMethods, advise, advise_all};
use quoin_auditor::catalog::{
    MemoryModuleCatalogSource, MethodCatalog, VerificationMethod, load_method_catalog,
};
use quoin_auditor::{ObligationEvidence, ObligationFacts, characteristics_of};
use quoin_quire_types::{CoverageDiagnostic, Obligation};

fn method(id: &str, class: &str, rules: &[(&str, &[&str])]) -> VerificationMethod {
    VerificationMethod {
        id: id.to_owned(),
        name: id.to_owned(),
        class: class.to_owned(),
        definition: String::new(),
        evidence_kind: None,
        applicability: rules
            .iter()
            .map(|(rule, values)| {
                (
                    (*rule).to_owned(),
                    values.iter().map(|value| (*value).to_owned()).collect(),
                )
            })
            .collect(),
        tooling: Vec::new(),
        module_name: "m".to_owned(),
    }
}

fn catalog_of(methods: Vec<VerificationMethod>) -> MethodCatalog {
    MethodCatalog {
        methods,
        duplicates: Vec::new(),
        unreadable: Vec::new(),
    }
}

fn facts(statement: &str) -> ObligationFacts {
    ObligationFacts {
        id: "FR-9-AC-1".to_owned(),
        statement: statement.to_owned(),
        ..ObligationFacts::default()
    }
}

fn of(statement: &str) -> Vec<String> {
    characteristics_of(statement, None, None, None)
}

fn has(found: &[String], name: &str) -> bool {
    found.iter().any(|candidate| candidate == name)
}

fn temporal_catalog() -> MethodCatalog {
    catalog_of(vec![
        method(
            "model-checking",
            "Analysis",
            &[("characteristics", &["temporal"])],
        ),
        method(
            "unit-testing",
            "Test",
            &[("characteristics", &["temporal"])],
        ),
    ])
}

/// Reliability phrasing is recommended fault injection.
#[trace("FR-031-AC-4")]
#[test]
fn reliability_phrasing_recommends_fault_injection() {
    let catalog = catalog_of(vec![
        method(
            "fault-injection",
            "Test",
            &[("characteristics", &["reliability"])],
        ),
        method("fuzzing", "Test", &[("characteristics", &["parser"])]),
    ]);
    let advice = advise(
        &catalog,
        &facts("The service SHALL tolerate and recover from a dependency outage."),
    );
    let methods: Vec<&str> = advice
        .recommended
        .iter()
        .map(|r| r.method.as_str())
        .collect();
    assert_eq!(methods, ["fault-injection"]);
}

/// A round-trip property shape recommends property-based and metamorphic
/// testing, ordered by how many rules matched.
#[trace("FR-031-AC-5")]
#[test]
fn a_round_trip_shape_recommends_property_based_then_metamorphic() {
    let catalog = catalog_of(vec![
        method(
            "metamorphic-testing",
            "Test",
            &[("property_shapes", &["round-trip"])],
        ),
        method(
            "property-based-testing",
            "Test",
            &[
                ("property_shapes", &["round-trip"]),
                ("characteristics", &["serialization"]),
            ],
        ),
    ]);
    let mut shaped = facts("decoding an encoded value round-trips");
    shaped.property_shape = Some("round-trip".to_owned());
    let advice = advise(&catalog, &shaped);
    let methods: Vec<&str> = advice
        .recommended
        .iter()
        .map(|r| r.method.as_str())
        .collect();
    assert_eq!(methods, ["property-based-testing", "metamorphic-testing"]);
    assert_eq!(advice.recommended[0].reasons.len(), 2);
    assert_eq!(advice.recommended[1].reasons.len(), 1);
}

/// An authored method no recommendation covers is a mismatch; one matching a
/// recommended method's class is not.
#[trace("FR-031-AC-6")]
#[test]
fn a_mismatch_needs_an_authored_method_no_recommendation_covers() {
    let mut authored = facts("the invariant always holds");
    authored.authored_method = Some("Inspection".to_owned());
    assert!(advise(&temporal_catalog(), &authored).mismatch);

    authored.authored_method = Some("Test".to_owned());
    assert!(!advise(&temporal_catalog(), &authored).mismatch);
    authored.authored_method = Some("model-checking".to_owned());
    assert!(!advise(&temporal_catalog(), &authored).mismatch);
}

/// An obligation matching no rule is inconclusive, with no recommendation and
/// no mismatch.
#[trace("FR-031-AC-7")]
#[test]
fn an_obligation_matching_no_rule_is_inconclusive() {
    let mut quiet = facts("nothing lexical here at all");
    quiet.authored_method = Some("Inspection".to_owned());
    let advice = advise(&temporal_catalog(), &quiet);
    assert!(advice.inconclusive);
    assert_eq!(advice.recommended.len(), 0);
    assert!(!advice.mismatch);
}

/// A method naming an axis the advisor cannot observe is still recommended on
/// the axes it can, with only the observable reasons reported.
#[trace("FR-031-AC-8")]
#[test]
fn an_unobservable_axis_is_skipped_not_fatal() {
    let catalog = catalog_of(vec![method(
        "model-checking",
        "Analysis",
        &[
            ("characteristics", &["temporal"]),
            ("phase_of_the_moon", &["waxing"]),
        ],
    )]);
    let advice = advise(&catalog, &facts("the invariant always holds"));
    assert_eq!(advice.recommended.len(), 1);
    let reasons: Vec<&str> = advice.recommended[0]
        .reasons
        .iter()
        .map(|r| r.rule.as_str())
        .collect();
    assert_eq!(reasons, ["characteristics"]);
}

/// A module whose manifest cannot be read or parsed is skipped and reported on
/// the merged catalog, never thrown.
#[trace("FR-031-AC-9")]
#[test]
fn an_unreadable_module_is_skipped_and_reported_not_thrown() {
    let good =
        "name: good\nverification_catalog:\n  unit-testing:\n    name: Unit\n    class: Test\n";
    let source = MemoryModuleCatalogSource::new()
        .with_unreadable("/broken", "EACCES")
        .with_module("/malformed", "a: [1,\nb: 2\n")
        .with_module("/good", good);
    let merged = load_method_catalog(&source);
    let reported: Vec<&str> = merged
        .unreadable
        .iter()
        .map(|m| m.module_root.as_str())
        .collect();
    assert_eq!(reported, ["/broken", "/malformed"]);
    let ids: Vec<&str> = merged.methods.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["unit-testing"]);
}

/// A link's target contributes nothing to characteristics; its text does.
#[trace("FR-031-AC-14")]
#[test]
fn a_link_target_contributes_nothing_but_its_text_does() {
    assert!(!has(
        &of("The result is written beside [the report](../stakeholder/StR-005-x.md)."),
        "stakeholder-facing"
    ));
    assert!(has(
        &of("The result is shown to the [customer](../report/R-5.md)."),
        "stakeholder-facing"
    ));
}

/// `high-criticality` is minted from the declared value, never from a
/// threshold the engine chose.
#[trace("FR-031-AC-15")]
#[test]
fn high_criticality_comes_from_the_declared_value() {
    let minted = |value: Option<&str>| {
        has(
            &characteristics_of("the thing works", value, None, None),
            "high-criticality",
        )
    };
    for declared in ["P0", "high", "critical", "HIGH"] {
        assert!(minted(Some(declared)), "{declared}");
    }
    for declared in ["P1", "medium", "low", "urgent", ""] {
        assert!(!minted(Some(declared)), "{declared:?}");
    }
    assert!(!minted(None));
}

/// A magic-value comparison, constant-time behaviour or reference equivalence
/// mints its characteristic; a function signature and a recorded-snapshot
/// comparison mint neither the first nor the last.
#[trace("FR-031-AC-17")]
#[test]
fn magic_value_constant_time_and_reference_equivalence_are_minted() {
    assert!(has(
        &of("The loader SHALL verify the checksum of each block."),
        "magic-value-comparison"
    ));
    assert!(has(
        &of("The comparison SHALL run in constant-time."),
        "secret-dependent-branch"
    ));
    assert!(has(
        &of("The port SHALL be behaviourally identical to the reference implementation."),
        "reference-equivalence"
    ));
    for neutral in [
        "The function signature SHALL accept a slice and return a count.",
        "The output SHALL be compared with the recorded snapshot.",
    ] {
        let found = of(neutral);
        assert!(
            !has(&found, "magic-value-comparison"),
            "{neutral}: {found:?}"
        );
        assert!(
            !has(&found, "reference-equivalence"),
            "{neutral}: {found:?}"
        );
    }
}

/// Fault-detection characteristics follow the evidence: bound with no score is
/// unmeasured, a weakest score below 1 is failed; unbound and unconsulted mint
/// neither.
#[trace("FR-031-AC-18")]
#[test]
fn fault_detection_characteristics_follow_the_evidence() {
    let with = |evidence: Option<ObligationEvidence>| {
        characteristics_of("the thing works", None, evidence.as_ref(), None)
    };
    let evidence = |bound: bool, scores: &[f64]| {
        Some(ObligationEvidence {
            bound,
            fault_detection_scores: scores.to_vec(),
        })
    };
    assert_eq!(with(evidence(true, &[])), ["fault-detection-unmeasured"]);
    assert_eq!(
        with(evidence(true, &[1.0, 0.9])),
        ["fault-detection-failed"]
    );
    assert_eq!(with(evidence(true, &[1.0, 1.0])).len(), 0);
    assert_eq!(with(evidence(false, &[])).len(), 0);
    assert_eq!(with(evidence(false, &[0.1])).len(), 0);
    assert_eq!(with(None).len(), 0);
}

/// The obligation's structured `parameters` reach the advisor: a `target` or
/// `threshold` key mints `quantified-threshold` whatever the wording, and other
/// keys mint nothing.
#[trace("FR-031-AC-19")]
#[test]
fn structured_parameters_mint_a_quantified_threshold() {
    let parameters = |key: &str| BTreeMap::from([(key.to_owned(), "< 4 min".to_owned())]);
    for key in ["target", "threshold"] {
        let found = characteristics_of("the thing works", None, None, Some(&parameters(key)));
        assert_eq!(found, ["quantified-threshold"], "{key}");
    }
    let other = characteristics_of("the thing works", None, None, Some(&parameters("owner")));
    assert!(other.is_empty(), "{other:?}");
}

/// A hyphenated compound is one token: a keyword that is a fragment of one
/// mints nothing, while a pattern naming the whole compound still matches.
#[trace("FR-031-AC-20")]
#[test]
fn a_hyphenated_compound_is_one_token() {
    assert!(!has(
        &of("The unsafe-audit step SHALL finish."),
        "memory-safety"
    ));
    assert!(has(&of("The crate SHALL be memory-safe."), "memory-safety"));
    assert!(has(
        &of("The queue SHALL guarantee thread-safety."),
        "concurrent"
    ));
    assert!(has(
        &of("The buffer SHALL prevent use-after-free."),
        "memory-safety"
    ));
}

/// A statement claiming an architectural fact in the corpus's own wording mints
/// `layering` or `module-boundary`; `top-level claim` mints neither.
#[trace("FR-031-AC-21")]
#[test]
fn architectural_claims_mint_layering_or_module_boundary() {
    assert!(has(
        &of("The crate SHALL have zero dependencies on the CLI."),
        "layering"
    ));
    assert!(has(
        &of("The dependency check SHALL reject an upward import."),
        "layering"
    ));
    assert!(has(&of("The crate graph SHALL be acyclic."), "layering"));
    assert!(has(
        &of("The engine SHALL stay absent from the top-level crates."),
        "module-boundary"
    ));
    let claim = of("The top-level claim holds for every release.");
    assert!(
        !has(&claim, "layering") && !has(&claim, "module-boundary"),
        "{claim:?}"
    );
}

fn obligation(id: &str, statement: &str, method: Option<&str>) -> Obligation {
    Obligation {
        id: id.to_owned(),
        statement: statement.to_owned(),
        target_ids: None,
        method: method.map(ToOwned::to_owned),
        criticality: None,
        parameters: None,
    }
}

fn uncatalogued(values: &[&str], degraded: bool) -> UncataloguedMethods {
    UncataloguedMethods {
        values: values
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<BTreeSet<_>>(),
        degraded,
    }
}

/// Uncatalogued, mismatch and inconclusive are three states: an uncatalogued
/// value is never a mismatch, a declared value the advisor disagrees with is,
/// and the uncatalogued fact survives silence.
#[trace("FR-031-AC-22")]
#[test]
fn uncatalogued_mismatch_and_inconclusive_are_three_states() {
    let obligations = [
        obligation("O-1", "the invariant always holds", Some("Smoke")),
        obligation("O-2", "the invariant always holds", Some("Inspection")),
        obligation("O-3", "nothing lexical here at all", Some("Smoke")),
        obligation("O-4", "the invariant always holds", Some("smoke")),
    ];
    let advice = advise_all(
        &temporal_catalog(),
        &obligations,
        &BTreeMap::<String, PropertyShape>::new(),
        &[],
        &[],
        &uncatalogued(&["Smoke"], false),
    );
    let state = |at: usize| {
        let a = &advice[at];
        (a.uncatalogued, a.mismatch, a.inconclusive)
    };
    assert_eq!(
        state(0),
        (true, false, false),
        "uncatalogued, never a mismatch"
    );
    assert_eq!(
        state(1),
        (false, true, false),
        "a declared value that disagrees"
    );
    assert_eq!(state(2), (true, false, true), "the fact survives silence");
    assert_eq!(state(3), (false, true, false), "the join is byte-equal");
}

/// Diagnostics with no `value` (an engine predating the classification) degrade
/// the advisor to two states; no such diagnostics at all is not degraded.
#[trace("FR-031-AC-23")]
#[test]
fn a_valueless_diagnostic_degrades_and_silence_does_not() {
    use quoin_auditor::uncatalogued_authored_methods;
    let reason = quoin_auditor::UNCATALOGUED_METHOD_REASON.to_owned();
    let valueless = [CoverageDiagnostic {
        reason: reason.clone(),
        value: None,
    }];
    let found = uncatalogued_authored_methods(&valueless);
    assert!(found.degraded);
    assert_eq!(found.values.len(), 0);

    let none = uncatalogued_authored_methods(&[]);
    assert!(!none.degraded);
    let unrelated = [CoverageDiagnostic {
        reason: "something-else".to_owned(),
        value: None,
    }];
    assert!(!uncatalogued_authored_methods(&unrelated).degraded);

    let valued = [CoverageDiagnostic {
        reason,
        value: Some("Smoke".to_owned()),
    }];
    let found = uncatalogued_authored_methods(&valued);
    assert!(!found.degraded);
    assert!(found.values.contains("Smoke"));
}

/// A declared configuration space advises the combinatorial method
/// structurally, whatever its words; ordinary prose does not.
#[trace("FR-035-AC-12")]
#[test]
fn a_declared_space_is_a_configuration_matrix_structurally() {
    let space = "2-way over features(default|python|wasm) target(linux|wasm32)";
    assert!(has(&of(space), "configuration-matrix"));
    assert!(!has(
        &of("The parser SHALL reject bad input."),
        "configuration-matrix"
    ));

    let catalog = catalog_of(vec![method(
        "combinatorial-testing",
        "Test",
        &[("characteristics", &["configuration-matrix"])],
    )]);
    let advice = advise(&catalog, &facts(space));
    assert_eq!(advice.recommended.len(), 1);
    assert!(advise(&catalog, &facts("The parser SHALL reject bad input.")).inconclusive);
}
