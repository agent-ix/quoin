// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `MeasurementPlan` loader unit fixtures.

#[cfg(feature = "campaign")]
use super::load_selected_measurement_plans;
use super::{PlanLoadOptions, load_measurement_plans};
use crate::error::MeasurementErrorCode;
use crate::source::MemoryMeasurement;
use crate::types::plan::{LifecycleStatus, MeasurementStage};
#[cfg(feature = "campaign")]
use std::collections::BTreeSet;

fn document(id: &str, metric: &str, stage: &str) -> String {
    format!(
        "---\ntype: MeasurementPlan\nid: {id}\ntitle: T\nstatus: active\nstage: \
             {stage}\nmetric: {metric}\ndefinition_version: v1\nowner: o\n---\n\n# {id}\n"
    )
}

#[cfg(feature = "campaign")]
#[test]
fn campaign_scope_validates_selected_plan_and_ignores_unrelated_legacy_semantics() {
    let legacy = "---\ntype: MeasurementPlan\nid: MP-OLD\ntitle: Legacy\nstatus: active\nstage: observe\nmetric: legacy\ndefinition_version: v1\nstatistical_design:\n  estimator: prose from an older schema\n---\n";
    let selected = document("MP-NEW", "gate", "ratchet");
    let source = MemoryMeasurement::new()
        .with_document("spec/assurance/old.md", legacy)
        .with_document("spec/assurance/new.md", selected);
    let ids = BTreeSet::from(["MP-NEW"]);
    let plans = load_selected_measurement_plans(&source, &ids).unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].id.as_str(), "MP-NEW");
    assert!(load_measurement_plans(&source, PlanLoadOptions::default()).is_err());

    let selected_invalid = BTreeSet::from(["MP-OLD"]);
    let error = load_selected_measurement_plans(&source, &selected_invalid).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
}

#[test]
fn plans_come_back_sorted_by_metric_then_id() {
    let source = MemoryMeasurement::new()
        .with_document("spec/assurance/b.md", document("MP-2", "zeta", "observe"))
        .with_document("spec/assurance/a.md", document("MP-3", "alpha", "ratchet"))
        .with_document("assurance/c.md", document("MP-1", "alpha", "trend"));
    let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    let order: Vec<&str> = plans.iter().map(|plan| plan.id.as_str()).collect();
    assert_eq!(order, ["MP-1", "MP-3", "MP-2"]);
    assert_eq!(plans[0].stage, MeasurementStage::Trend);
    assert_eq!(plans[0].status, LifecycleStatus::Active);
}

/// Trace: FR-114-AC-1
/// Provenance: PLAT-1043
#[test]
fn tc_1940_runnable_procedure_must_be_a_protected_repo_file() {
    let front = "---\ntype: MeasurementPlan\nid: MP-1\ntitle: T\nstatus: active\
            \nstage: observe\nmetric: m\ndefinition_version: v1\
            \nexecution_procedure: spec/assurance/procedure.json\n";
    let protected =
        format!("{front}protected_apparatus:\n  - spec/assurance/procedure.json\n---\n\n# T\n");
    let plans = load_measurement_plans(
        &MemoryMeasurement::new().with_document("spec/assurance/MP-1.md", protected),
        PlanLoadOptions::default(),
    )
    .expect("protected procedure is runnable");
    assert_eq!(
        plans[0].execution_procedure.as_deref(),
        Some("spec/assurance/procedure.json")
    );

    let unprotected = format!("{front}---\n\n# T\n");
    let error = load_measurement_plans(
        &MemoryMeasurement::new().with_document("spec/assurance/MP-1.md", unprotected),
        PlanLoadOptions::default(),
    )
    .expect_err("unprotected procedure cannot be used");
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
}

#[test]
fn governance_members_are_carried_only_when_asked_for() {
    let source =
        MemoryMeasurement::new().with_document("assurance/a.md", document("MP-1", "m", "ratchet"));
    let without = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    assert_eq!(without[0].owner, None);
    let with = load_measurement_plans(
        &source,
        PlanLoadOptions {
            include_governance: true,
        },
    )
    .unwrap();
    assert_eq!(with[0].owner.as_deref(), Some("o"));
}

#[test]
fn a_document_of_another_type_is_skipped_and_a_bad_stage_is_refused() {
    let other = MemoryMeasurement::new()
        .with_document(
            "assurance/a.md",
            "---\ntype: AssuranceProfile\nid: AP-1\n---\n",
        )
        .with_document("assurance/b.md", "# no frontmatter\n");
    assert!(
        load_measurement_plans(&other, PlanLoadOptions::default())
            .unwrap()
            .is_empty()
    );

    let bad = MemoryMeasurement::new()
        .with_document("assurance/a.md", document("MP-1", "m", "speculate"));
    let error = load_measurement_plans(&bad, PlanLoadOptions::default()).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
    assert!(error.subject().contains("unknown measurement stage"));
}
