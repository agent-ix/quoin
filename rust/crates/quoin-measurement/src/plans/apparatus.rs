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
use crate::types::plan::MeasurementStage;

use super::CODE;

/// Read the optional `protected_apparatus` and `negative_controls`, each
/// `None` when the document does not state it.
///
/// Engineering-assurance's schema rules that EA's types cannot carry are
/// enforced here too (FR-024-AC-8): a `gate` plan must state both lists, and
/// an `apparatus-edit` control over no declared apparatus guards nothing.
/// There is no exception for a gate plan written before FR-024: a gate that
/// names nothing it protects gives credit a changed answer key can earn.
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] when either member is
/// present and EA refuses it, naming the member and carrying EA's reason,
/// when a `gate` plan states either list not at all, and when an
/// `apparatus-edit` control is declared with no `protected_apparatus`.
pub(super) fn apparatus_from(
    path: &str,
    value: &serde_json::Value,
    stage: MeasurementStage,
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
    if stage == MeasurementStage::Gate {
        for (member, missing) in [
            ("protected_apparatus", protected.is_none()),
            ("negative_controls", controls.is_none()),
        ] {
            if missing {
                return Err(MeasurementError::new(
                    CODE,
                    format!("{path}: a `gate` plan requires `{member}`"),
                ));
            }
        }
    }
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
