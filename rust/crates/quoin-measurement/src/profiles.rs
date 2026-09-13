// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Loading active `AssuranceProfile` summaries out of frontmatter.
//!
//! Ports `src/measurement/profiles.ts`. The walk is
//! [`crate::discovery::documents`], shared with [`crate::plans`].

use crate::discovery;
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::source::MeasurementSource;
use crate::types::ids::NonEmptyText;
use crate::types::plan::LifecycleStatus;
use crate::types::profile::AssuranceProfileSummary;

/// The code every refusal in this module carries.
const CODE: MeasurementErrorCode = MeasurementErrorCode::ProfileInvalid;

/// Load every **active** `AssuranceProfile` the assurance roots declare.
///
/// The result is sorted by id, then path (`profiles.ts:23`). A proposed or
/// retired profile is dropped after it has been read, so a malformed retired
/// document still refuses — that is `profiles.ts:19-24`'s order and it is kept.
///
/// # Errors
///
/// [`MeasurementErrorCode::ProfileInvalid`] when an `AssuranceProfile` document
/// lacks a required member or names an unknown status;
/// [`MeasurementErrorCode::Yaml`] for unreadable frontmatter; and
/// [`MeasurementErrorCode::Io`] when a document cannot be read.
pub fn load_active_assurance_profiles<S: MeasurementSource + ?Sized>(
    source: &S,
) -> Result<Vec<AssuranceProfileSummary>, MeasurementError> {
    let mut profiles = discovery::documents(source, "AssuranceProfile", profile_from)?;
    profiles.retain(|profile| profile.status == LifecycleStatus::Active);
    profiles.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    Ok(profiles)
}

fn profile_from(
    path: &str,
    value: &serde_json::Value,
) -> Result<AssuranceProfileSummary, MeasurementError> {
    let required = |name: &str| -> Result<NonEmptyText, MeasurementError> {
        discovery::required(value, name)
            .ok_or_else(|| {
                MeasurementError::new(
                    CODE,
                    format!("{path}: AssuranceProfile requires non-empty `{name}`"),
                )
            })
            .and_then(|text| NonEmptyText::parse(text, CODE, name))
    };
    let id = required("id")?;
    let title = required("title")?;
    let status = required("status")?;
    let status = LifecycleStatus::from_wire(status.as_str()).ok_or_else(|| {
        MeasurementError::new(
            CODE,
            format!("{path}: unknown AssuranceProfile status `{status}`"),
        )
    })?;
    Ok(AssuranceProfileSummary {
        id,
        title,
        status,
        path: path.to_owned(),
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
    use super::load_active_assurance_profiles;
    use crate::error::MeasurementErrorCode;
    use crate::source::MemoryMeasurement;

    fn document(id: &str, status: &str) -> String {
        format!("---\ntype: AssuranceProfile\nid: {id}\ntitle: T\nstatus: {status}\n---\n")
    }

    #[test]
    fn only_active_profiles_come_back_and_they_are_sorted_by_id_then_path() {
        let source = MemoryMeasurement::new()
            .with_document("spec/assurance/b.md", document("AP-2", "active"))
            .with_document("spec/assurance/a.md", document("AP-1", "active"))
            .with_document("assurance/c.md", document("AP-3", "retired"));
        let profiles = load_active_assurance_profiles(&source).unwrap();
        let order: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(order, ["AP-1", "AP-2"]);
    }

    #[test]
    fn a_malformed_retired_profile_still_refuses() {
        let source =
            MemoryMeasurement::new().with_document("assurance/a.md", document("AP-1", "sunset"));
        let error = load_active_assurance_profiles(&source).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ProfileInvalid);
        assert!(error.subject().contains("unknown AssuranceProfile status"));
    }
}
