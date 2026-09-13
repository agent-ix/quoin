// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Loading `MeasurementPlan`s out of assurance-document frontmatter.
//!
//! Ports `src/measurement/plans.ts`. The walk itself is
//! [`crate::discovery::documents`] — see that module for why it is shared with
//! [`crate::profiles`] rather than written twice.

use crate::discovery;
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::source::MeasurementSource;
use crate::types::ids::NonEmptyText;
use crate::types::plan::{LifecycleStatus, MeasurementPlan, MeasurementStage};

/// The code every refusal in this module carries.
const CODE: MeasurementErrorCode = MeasurementErrorCode::PlanInvalid;

/// What a plan load should include beyond the members every caller needs.
///
/// `plans.ts:20` takes `{ includeGovernance?: boolean }` and spreads `owner`
/// and `action` in only when it is set, so a caller that did not ask cannot
/// read them by accident.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlanLoadOptions {
    /// Carry `owner` and `action` through when the document states them.
    pub include_governance: bool,
}

/// Load every `MeasurementPlan` the repository's assurance roots declare.
///
/// The result is sorted by metric, then id (`plans.ts:25`).
///
/// # Errors
///
/// [`MeasurementErrorCode::PlanInvalid`] when a `MeasurementPlan` document
/// lacks a required member, or names an unknown stage or status;
/// [`MeasurementErrorCode::Yaml`] for unreadable frontmatter; and
/// [`MeasurementErrorCode::Io`] when a document cannot be read.
pub fn load_measurement_plans<S: MeasurementSource + ?Sized>(
    source: &S,
    options: PlanLoadOptions,
) -> Result<Vec<MeasurementPlan>, MeasurementError> {
    let mut plans = discovery::documents(source, "MeasurementPlan", |path, value| {
        plan_from(path, value, options)
    })?;
    // Stable, so documents that agree on metric and id keep the walk's own
    // path order, as `Array.prototype.sort` does in V8.
    plans.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    Ok(plans)
}

fn plan_from(
    path: &str,
    value: &serde_json::Value,
    options: PlanLoadOptions,
) -> Result<MeasurementPlan, MeasurementError> {
    let required = |name: &str| -> Result<NonEmptyText, MeasurementError> {
        discovery::required(value, name)
            .ok_or_else(|| {
                MeasurementError::new(
                    CODE,
                    format!("{path}: MeasurementPlan requires non-empty `{name}`"),
                )
            })
            .and_then(|text| NonEmptyText::parse(text, CODE, name))
    };
    // `plans.ts:44-56` checks the six required members before it looks at the
    // stage or the status, so a document missing `stage` entirely is refused
    // for the missing member rather than for an unknown stage.
    let id = required("id")?;
    let title = required("title")?;
    let status = required("status")?;
    let stage = required("stage")?;
    let metric = required("metric")?;
    let definition_version = required("definition_version")?;

    let stage = MeasurementStage::from_wire(stage.as_str()).ok_or_else(|| {
        MeasurementError::new(CODE, format!("{path}: unknown measurement stage `{stage}`"))
    })?;
    let status = LifecycleStatus::from_wire(status.as_str()).ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!("{path}: unknown MeasurementPlan status `{status}`"),
        )
    })?;

    let governance = |name: &str| -> Option<String> {
        if options.include_governance {
            discovery::required(value, name).map(str::to_owned)
        } else {
            None
        }
    };
    Ok(MeasurementPlan {
        id,
        title,
        status,
        stage,
        metric,
        definition_version,
        path: path.to_owned(),
        owner: governance("owner"),
        action: governance("action"),
    })
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use super::{PlanLoadOptions, load_measurement_plans};
    use crate::error::MeasurementErrorCode;
    use crate::source::MemoryMeasurement;
    use crate::types::plan::{LifecycleStatus, MeasurementStage};

    fn document(id: &str, metric: &str, stage: &str) -> String {
        format!(
            "---\ntype: MeasurementPlan\nid: {id}\ntitle: T\nstatus: active\nstage: \
             {stage}\nmetric: {metric}\ndefinition_version: v1\nowner: o\n---\n\n# {id}\n"
        )
    }

    #[test]
    fn plans_come_back_sorted_by_metric_then_id() {
        let source = MemoryMeasurement::new()
            .with_document("spec/assurance/b.md", document("MP-2", "zeta", "observe"))
            .with_document("spec/assurance/a.md", document("MP-3", "alpha", "gate"))
            .with_document("assurance/c.md", document("MP-1", "alpha", "trend"));
        let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
        let order: Vec<&str> = plans.iter().map(|plan| plan.id.as_str()).collect();
        assert_eq!(order, ["MP-1", "MP-3", "MP-2"]);
        assert_eq!(plans[0].stage, MeasurementStage::Trend);
        assert_eq!(plans[0].status, LifecycleStatus::Active);
    }

    #[test]
    fn governance_members_are_carried_only_when_asked_for() {
        let source =
            MemoryMeasurement::new().with_document("assurance/a.md", document("MP-1", "m", "gate"));
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
}
