// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Vocabulary-completeness findings (FR-037), asserted against declarations and
//! documents built in each test. Nothing here replays a captured corpus.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_completeness::{
    ArtifactTypeName, DocumentClaims, FindingKind, FrontmatterField, Severity,
    VocabularyDeclaration, VocabularyName, assess_vocabulary,
};

fn declaration() -> VocabularyDeclaration {
    VocabularyDeclaration {
        name: VocabularyName::new("quality"),
        from: ArtifactTypeName::new("NFR"),
        field: FrontmatterField::new("quality_attribute"),
        check: "quality-coverage".to_owned(),
        justified_absence_field: Some("excluded".to_owned()),
        values: ["speed", "safety", "reach"].map(str::to_owned).to_vec(),
        module_name: "m".to_owned(),
    }
}

fn document(claims: &[&str], excuses: &[&str], body: &str) -> DocumentClaims {
    DocumentClaims {
        path: "doc.md".to_owned(),
        claims: claims.iter().map(|c| (*c).to_owned()).collect(),
        excuses: excuses.iter().map(|c| (*c).to_owned()).collect(),
        body: body.to_owned(),
    }
}

#[trace("FR-037-AC-3")]
#[test]
fn tc_208_an_unclaimed_value_is_an_unowned_gap_at_medium() {
    let result = assess_vocabulary(&declaration(), &[document(&["speed", "safety"], &[], "")]);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].value.as_str(), "reach");
    assert_eq!(result.findings[0].kind, FindingKind::Unowned);
    assert_eq!(result.findings[0].severity, Severity::Medium);
    assert_eq!(result.rollup.unowned, 1);
}

#[trace("FR-037-AC-4")]
#[test]
fn tc_209_an_exclusion_with_no_written_reason_is_high() {
    let doc = document(&["speed", "reach"], &["safety"], "no table here\n");
    let result = assess_vocabulary(&declaration(), &[doc]);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].kind, FindingKind::UnjustifiedExclusion);
    assert_eq!(result.findings[0].severity, Severity::High);
    assert!(result.findings[0].severity > Severity::Medium);
}

#[trace("FR-037-AC-5")]
#[test]
fn tc_210_an_exclusion_outside_the_vocabulary_excuses_nothing() {
    let doc = document(
        &["speed", "reach"],
        &["saftey"],
        "| saftey | controls no hardware |\n",
    );
    let result = assess_vocabulary(&declaration(), &[doc]);
    let kinds: Vec<FindingKind> = result.findings.iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&FindingKind::UndeclaredExclusion));
    // The real value keeps reporting.
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.kind == FindingKind::Unowned && f.value.as_str() == "safety")
    );
    assert_eq!(result.rollup.excused, 0);
}

#[trace("FR-037-AC-6")]
#[test]
fn tc_211_a_table_row_with_a_real_reason_accepts_the_exclusion() {
    let doc = document(
        &["speed", "reach"],
        &["safety"],
        "| safety | this product controls no hardware |\n",
    );
    let result = assess_vocabulary(&declaration(), &[doc]);
    assert_eq!(result.findings.len(), 0);
    assert_eq!(result.rollup.excused, 1);
}
