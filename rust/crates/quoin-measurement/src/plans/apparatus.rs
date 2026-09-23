// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A plan's `protected_apparatus` and `negative_controls` (PLAT-975).
//!
//! Both are engineering-assurance's (its FR-024) and are deserialized into
//! EA's own [`ProtectedApparatus`] and [`NegativeControls`], so the entry
//! grammar, the list rules and the control kinds are stated once, there.
//! Split from [`super`] so the plan walk stays under this crate's module-size
//! ceiling.

use engineering_assurance::measurement::{
    NegativeControlKind, NegativeControls, ProtectedApparatus,
};
use serde::Deserialize;

use crate::error::MeasurementError;

use super::CODE;

/// Read the optional `protected_apparatus` and `negative_controls`, each
/// `None` when the document does not state it.
///
/// An `apparatus-edit` control over no declared apparatus guards nothing, so
/// it is refused here, as EA's schema refuses it (FR-024-AC-8). The gate-stage
/// requirement for both lists is the schema's alone: EA's types carry no
/// stage, and a gate plan written before FR-024 still loads.
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] when either member is
/// present and EA refuses it, naming the member and carrying EA's reason, or
/// when an `apparatus-edit` control is declared with no `protected_apparatus`.
pub(super) fn apparatus_from(
    path: &str,
    value: &serde_json::Value,
) -> Result<(Option<ProtectedApparatus>, Option<NegativeControls>), MeasurementError> {
    let protected = value
        .get("protected_apparatus")
        .map(|stated| {
            ProtectedApparatus::deserialize(stated).map_err(|error| {
                MeasurementError::new(
                    CODE,
                    format!("{path}: protected_apparatus is invalid: {error}; found {stated}"),
                )
            })
        })
        .transpose()?;
    let controls = value
        .get("negative_controls")
        .map(|stated| {
            NegativeControls::deserialize(stated).map_err(|error| {
                MeasurementError::new(
                    CODE,
                    format!("{path}: negative_controls is invalid: {error}; found {stated}"),
                )
            })
        })
        .transpose()?;
    if protected.is_none()
        && controls
            .as_ref()
            .is_some_and(|controls| controls.covers(NegativeControlKind::ApparatusEdit))
    {
        return Err(MeasurementError::new(
            CODE,
            format!(
                "{path}: negative_controls is invalid: an `apparatus-edit` control requires a \
                 `protected_apparatus` list for it to guard"
            ),
        ));
    }
    Ok((protected, controls))
}
