// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a plan's earlier revisions, for a caller that has git (PLAT-985).
//!
//! This crate's own walk only ever reads a repository's current tree through
//! a [`MeasurementSource`](crate::source::MeasurementSource).
//! engineering-assurance's `definition_change_without_version_bump` check
//! (its FR-021) needs a plan's *earlier* revisions too, which only a caller
//! with git can read (`git show <commit>:<path>`) — this crate has no source
//! abstraction for another commit's tree, and the `quoin-core` boundary this
//! crate is read through carries no host capability at all. So the caller
//! does the git reading and hands this module the text; this module does the
//! parsing and the comparison, so the caller never has to construct
//! engineering-assurance's own types.

use crate::discovery;
use crate::error::MeasurementError;
use crate::plans::{PlanLoadOptions, plan_from};
use crate::types::plan::MeasurementPlan;

/// Parse one `MeasurementPlan` document's text on its own, without walking
/// the repository.
///
/// This is the same frontmatter parse [`crate::plans::load_measurement_plans`]
/// runs per file, exposed over a string instead of a walk, so a caller
/// reading a plan's git history can parse each revision's blob the same way.
///
/// `path` is used only for error messages. `Ok(None)` when `text` carries no
/// frontmatter, or frontmatter naming a different `type`, mirroring
/// [`discovery::documents`]'s own skip rule — a caller reading a plan's
/// history across a rename, or before the document existed, sees an absence
/// rather than an error.
///
/// # Errors
///
/// The same as [`crate::plans::load_measurement_plans`], for text whose
/// frontmatter names `type: MeasurementPlan` but is otherwise invalid.
pub fn plan_from_text(
    path: &str,
    text: &str,
    options: PlanLoadOptions,
) -> Result<Option<MeasurementPlan>, MeasurementError> {
    let Some(block) = discovery::frontmatter(text) else {
        return Ok(None);
    };
    let value = quoin_yaml::from_str(block)?;
    if value.get("type").and_then(serde_json::Value::as_str) != Some("MeasurementPlan") {
        return Ok(None);
    }
    plan_from(path, &value, text, options).map(Some)
}

/// Whether `before` and `after` — two revisions of the same plan document, in
/// git order, as a caller with git read from its history — changed the
/// objective, estimator, decision rule or protected apparatus while keeping
/// the same `definition_version` (engineering-assurance FR-021's
/// `definition_change_without_version_bump`).
///
/// EA's own function reads a genuine bump only as `after`'s version
/// differing from `before`'s, so the two must be given in git order (older
/// first) — comparing them the other way around would read every real bump
/// as a violation. This wraps EA's function so a caller only ever holds this
/// crate's own `MeasurementPlan`, never constructs EA's `PlanDefinition`
/// itself.
#[must_use]
pub fn definition_changed_without_version_bump(
    before: &MeasurementPlan,
    after: &MeasurementPlan,
) -> bool {
    engineering_assurance::measurement::definition_change_without_version_bump(
        &plan_definition_of(before),
        &plan_definition_of(after),
    )
    .is_some()
}

/// `plan`'s versioned measurement-definition members, in engineering-
/// assurance's own shape.
fn plan_definition_of(
    plan: &MeasurementPlan,
) -> engineering_assurance::measurement::PlanDefinition<'_> {
    use engineering_assurance::measurement::{MeasurementDefinition, PlanDefinition};

    PlanDefinition {
        definition_version: Some(plan.definition_version.as_str()),
        definition: MeasurementDefinition {
            objective: plan.objective,
            estimator: plan.statistical_design.and_then(|design| design.estimator),
            decision_rule: plan
                .statistical_design
                .and_then(|design| design.decision_rule),
            protected_apparatus: plan.protected_apparatus.clone(),
        },
    }
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
    use super::{definition_changed_without_version_bump, plan_from_text};
    use crate::plans::PlanLoadOptions;

    const DOC: &str = "---\nid: MP-1\ntitle: A plan\nstatus: active\nstage: observe\nmetric: m\ndefinition_version: v1\ntype: MeasurementPlan\nobjective:\n  direction: higher\n---\nbody\n";

    #[test]
    fn text_with_no_frontmatter_or_the_wrong_type_is_none() {
        assert!(
            plan_from_text("a.md", "no frontmatter here", PlanLoadOptions::default())
                .unwrap()
                .is_none()
        );
        let other = "---\ntype: AssuranceProfile\n---\n";
        assert!(
            plan_from_text("a.md", other, PlanLoadOptions::default())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_measurement_plan_document_parses() {
        let plan = plan_from_text("a.md", DOC, PlanLoadOptions::default())
            .unwrap()
            .expect("a MeasurementPlan document parses");
        assert_eq!(plan.id.as_str(), "MP-1");
    }

    #[test]
    fn an_objective_added_under_the_same_version_is_a_change_without_a_bump() {
        let before = "---\nid: MP-1\ntitle: A plan\nstatus: active\nstage: observe\nmetric: m\ndefinition_version: v1\ntype: MeasurementPlan\n---\nbody\n";
        let before = plan_from_text("a.md", before, PlanLoadOptions::default())
            .unwrap()
            .unwrap();
        let after = plan_from_text("a.md", DOC, PlanLoadOptions::default())
            .unwrap()
            .unwrap();
        assert!(definition_changed_without_version_bump(&before, &after));
    }

    #[test]
    fn the_same_edit_with_a_genuine_bump_is_not_a_violation() {
        let before = plan_from_text("a.md", DOC, PlanLoadOptions::default())
            .unwrap()
            .unwrap();
        let bumped = DOC
            .replace("definition_version: v1", "definition_version: v2")
            .replace("direction: higher", "direction: lower");
        let after = plan_from_text("a.md", &bumped, PlanLoadOptions::default())
            .unwrap()
            .unwrap();
        assert!(!definition_changed_without_version_bump(&before, &after));
    }

    #[test]
    fn an_unchanged_definition_is_not_a_violation() {
        let before = plan_from_text("a.md", DOC, PlanLoadOptions::default())
            .unwrap()
            .unwrap();
        let after = plan_from_text("a.md", DOC, PlanLoadOptions::default())
            .unwrap()
            .unwrap();
        assert!(!definition_changed_without_version_bump(&before, &after));
    }
}
