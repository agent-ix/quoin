// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Architecture advice honors the method catalog and keeps triggers distinct.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report"
)]

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use quoin_auditor::advise::{ObligationFacts, advise, characteristics_of};
use quoin_auditor::catalog::{MethodCatalog, VerificationMethod};

fn architecture_catalog() -> MethodCatalog {
    MethodCatalog {
        methods: vec![VerificationMethod {
            id: "architecture-conformance".to_owned(),
            name: "Architecture conformance".to_owned(),
            class: "Analysis".to_owned(),
            definition: "Check declared architectural boundaries.".to_owned(),
            evidence_kind: Some("static".to_owned()),
            unreachable_reason: None,
            applicability: BTreeMap::from([(
                "characteristics".to_owned(),
                vec!["layering".to_owned(), "module-boundary".to_owned()],
            )]),
            tooling: Vec::new(),
            module_name: "spec-artifacts-process".to_owned(),
        }],
        duplicates: Vec::new(),
        unreadable: Vec::new(),
    }
}

#[trace("FR-036-AC-6")]
#[test]
fn tc_fr036_architecture_conformance_matches_both_catalog_characteristics() {
    let statement = "The crate SHALL have zero dependencies on the CLI, and the engine SHALL stay absent from the top-level crates.";
    let facts = ObligationFacts {
        id: "FR-036-AC-6".to_owned(),
        statement: statement.to_owned(),
        ..ObligationFacts::default()
    };
    let characteristics: BTreeSet<String> = characteristics_of(statement, None, None, None)
        .into_iter()
        .collect();
    assert!(characteristics.contains("layering"), "{characteristics:?}");
    assert!(
        characteristics.contains("module-boundary"),
        "{characteristics:?}"
    );

    let advice = advise(&architecture_catalog(), &facts);
    assert_eq!(advice.recommended.len(), 1, "{advice:?}");
    let recommendation = &advice.recommended[0];
    assert_eq!(recommendation.method, "architecture-conformance");
    assert_eq!(
        recommendation
            .reasons
            .iter()
            .filter(|reason| reason.rule == "characteristics")
            .map(|reason| reason.value.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["layering", "module-boundary"])
    );
}

#[trace("FR-036-CON-3")]
#[test]
fn tc_fr036_layering_and_module_boundary_triggers_do_not_overlap() {
    let layering_phrases = [
        "depends on",
        "depending upon",
        "layering",
        "layered architecture",
        "acyclic",
        "dependency cycle",
        "dependency checks",
        "zero dependencies on",
        "no dependency on",
        "shall not import",
        "must not depend",
    ];
    let module_boundary_phrases = [
        "module boundary",
        "crate boundaries",
        "top-level module",
        "top-level crates",
        "remains absent from",
        "stays absent from",
        "public API",
        "public interface",
        "public surface",
        "internals of",
        "internal to",
        "exported from",
        "exported by",
    ];

    for phrase in layering_phrases {
        let found = characteristics_of(&format!("The statement names {phrase}."), None, None, None);
        assert!(
            found.iter().any(|name| name == "layering"),
            "{phrase}: {found:?}"
        );
        assert!(
            !found.iter().any(|name| name == "module-boundary"),
            "layering phrase `{phrase}` also minted module-boundary: {found:?}"
        );
    }
    for phrase in module_boundary_phrases {
        let found = characteristics_of(&format!("The statement names {phrase}."), None, None, None);
        assert!(
            found.iter().any(|name| name == "module-boundary"),
            "{phrase}: {found:?}"
        );
        assert!(
            !found.iter().any(|name| name == "layering"),
            "module-boundary phrase `{phrase}` also minted layering: {found:?}"
        );
    }
}
