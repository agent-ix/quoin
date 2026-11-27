// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Older FR-031 catalog and recommendation cases that were not directly tested.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_auditor::advise::{ObligationFacts, advise, mintable_characteristics};
use quoin_auditor::catalog::{
    MemoryModuleCatalogSource, MethodCatalog, VerificationMethod, load_method_catalog,
    unjustified_unreachable_methods, unreachable_characteristics,
};

fn method(id: &str, rules: &[(&str, &[&str])]) -> VerificationMethod {
    VerificationMethod {
        id: id.to_owned(),
        name: id.to_owned(),
        class: "Test".to_owned(),
        definition: String::new(),
        evidence_kind: None,
        applicability: rules
            .iter()
            .map(|(axis, values)| {
                (
                    (*axis).to_owned(),
                    values.iter().map(|value| (*value).to_owned()).collect(),
                )
            })
            .collect(),
        unreachable_reason: None,
        tooling: Vec::new(),
        module_name: "fixture".to_owned(),
    }
}

fn catalog(methods: Vec<VerificationMethod>) -> MethodCatalog {
    MethodCatalog {
        methods,
        duplicates: Vec::new(),
        unreadable: Vec::new(),
    }
}

#[trace("FR-031-AC-1")]
#[test]
fn tc_031_120_method_catalog_merge_preserves_rules_and_first_wins() {
    let first = r#"
name: alpha
verification_catalog:
  security-review:
    name: Security review
    class: Analysis
    definition: Review the declared attack surface.
    evidence_kind: review-report
    applicability:
      characteristics: [security]
      object_types: [attack_surface]
  unit-testing:
    name: Unit testing
    class: Test
    applicability:
      characteristics: [temporal]
"#;
    let second = r#"
name: beta
verification_catalog:
  unit-testing:
    name: Competing unit method
    class: Test
    applicability:
      characteristics: [serialization]
  model-checking:
    name: Model checking
    class: Analysis
"#;
    let source = MemoryModuleCatalogSource::new()
        .with_module("/first", first)
        .with_module("/second", second);
    let merged = load_method_catalog(&source);

    let ids: Vec<&str> = merged
        .methods
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(ids, ["model-checking", "security-review", "unit-testing"]);
    let unit = merged
        .methods
        .iter()
        .find(|entry| entry.id == "unit-testing")
        .expect("the first unit-testing declaration remains present");
    assert_eq!(unit.name, "Unit testing");
    assert_eq!(unit.module_name, "alpha");
    assert_eq!(unit.applicability["characteristics"], ["temporal"]);
    let security = merged
        .methods
        .iter()
        .find(|entry| entry.id == "security-review")
        .expect("the declared security method remains present");
    assert_eq!(security.evidence_kind.as_deref(), Some("review-report"));
    assert_eq!(security.applicability["object_types"], ["attack_surface"]);
    assert_eq!(merged.duplicates.len(), 1);
    assert_eq!(merged.duplicates[0].id, "unit-testing");
    assert_eq!(merged.duplicates[0].modules, ["alpha", "beta"]);

    let undeclared = load_method_catalog(&MemoryModuleCatalogSource::new());
    assert!(undeclared.methods.is_empty());
    assert!(undeclared.duplicates.is_empty());
}

#[trace("FR-031-AC-2")]
#[test]
fn tc_031_121_attack_surface_rules_recommend_security_methods() {
    let active = catalog(vec![
        method("dast", &[("object_types", &["attack_surface"])]),
        method(
            "negative-abuse-testing",
            &[("object_types", &["attack_surface"])],
        ),
        method("sast", &[("object_types", &["attack_surface"])]),
    ]);
    let facts = ObligationFacts {
        id: "FR-9-AC-1".to_owned(),
        statement: "The service accepts a declared attack surface.".to_owned(),
        object_types: vec!["attack_surface".to_owned()],
        ..ObligationFacts::default()
    };
    let advice = advise(&active, &facts);
    let recommended: Vec<&str> = advice
        .recommended
        .iter()
        .map(|entry| entry.method.as_str())
        .collect();
    assert_eq!(recommended, ["dast", "negative-abuse-testing", "sast"]);
}

#[trace("FR-031-AC-3")]
#[test]
fn tc_031_122_temporal_phrasing_recommends_monitoring_and_model_checking() {
    let active = catalog(vec![
        method("model-checking", &[("characteristics", &["temporal"])]),
        method("runtime-monitoring", &[("characteristics", &["temporal"])]),
    ]);
    let facts = ObligationFacts {
        id: "FR-9-AC-2".to_owned(),
        statement: "The service SHALL keep serving reads while the service is degraded.".to_owned(),
        ..ObligationFacts::default()
    };
    let advice = advise(&active, &facts);
    let recommended: Vec<&str> = advice
        .recommended
        .iter()
        .map(|entry| entry.method.as_str())
        .collect();
    assert_eq!(recommended, ["model-checking", "runtime-monitoring"]);
    assert_eq!(advice.recommended[0].reasons[0].value, "temporal");
}

#[trace("FR-031-AC-13")]
#[test]
fn tc_031_123_temporal_characteristic_is_mintable_and_reaches_its_method() {
    let mintable = mintable_characteristics();
    assert!(mintable.contains("temporal"));

    let active = catalog(vec![method(
        "model-checking",
        &[("characteristics", &["temporal"])],
    )]);
    let facts = ObligationFacts {
        id: "FR-9-AC-3".to_owned(),
        statement: "The service SHALL eventually recover.".to_owned(),
        ..ObligationFacts::default()
    };
    let advice = advise(&active, &facts);
    assert_eq!(advice.recommended.len(), 1);
    assert_eq!(advice.recommended[0].method, "model-checking");
}

#[trace("FR-031-AC-16")]
#[test]
fn tc_031_124_unmintable_characteristics_report_declaring_methods() {
    let active = catalog(vec![
        method(
            "first-method",
            &[("characteristics", &["removed-characteristic"])],
        ),
        method(
            "second-method",
            &[("characteristics", &["removed-characteristic"])],
        ),
        method("temporal-method", &[("characteristics", &["temporal"])]),
    ]);
    let findings = unreachable_characteristics(&active);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].characteristic, "removed-characteristic");
    assert_eq!(findings[0].methods, ["first-method", "second-method"]);
}

#[trace("FR-031-AC-13")]
#[test]
fn tc_031_125_characteristics_only_methods_need_a_nonempty_unreachable_reason() {
    let source = MemoryModuleCatalogSource::new().with_module(
        "/methods",
        r#"
name: methods
verification_catalog:
  unjustified:
    name: Unjustified
    class: Analysis
    applicability:
      characteristics: [never-produced]
  justified:
    name: Justified
    class: Analysis
    applicability:
      characteristics: [never-produced]
    unreachable_reason: Reserved for an input fact source added later.
  reachable:
    name: Reachable
    class: Analysis
    applicability:
      characteristics: [temporal]
  other-axis:
    name: Other axis
    class: Analysis
    applicability:
      characteristics: [never-produced]
      object_types: [attack_surface]
"#,
    );
    let catalog = load_method_catalog(&source);
    assert_eq!(
        catalog
            .methods
            .iter()
            .find(|method| method.id == "justified")
            .and_then(|method| method.unreachable_reason.as_deref()),
        Some("Reserved for an input fact source added later.")
    );
    assert_eq!(
        unjustified_unreachable_methods(&catalog)
            .into_iter()
            .map(|method| method.method)
            .collect::<Vec<_>>(),
        ["unjustified"]
    );
    let justified_only = MethodCatalog {
        methods: vec![method_with_reason(
            "justified",
            "never-produced",
            "Reserved for an input fact source added later.",
        )],
        duplicates: Vec::new(),
        unreadable: Vec::new(),
    };
    assert!(unreachable_characteristics(&justified_only).is_empty());
}

fn method_with_reason(id: &str, characteristic: &str, reason: &str) -> VerificationMethod {
    let mut method = method(id, &[("characteristics", &[characteristic])]);
    method.unreachable_reason = Some(reason.to_owned());
    method
}
