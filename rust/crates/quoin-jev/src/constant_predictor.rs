// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The constant-predictor baseline's producer half (quoin PLAT-1016).
//!
//! # What this module is, and what it is not
//!
//! MP-222/PLAT-932's constant-predictor baseline — the size-weighted mean of
//! the best-constant agreement per answer family, `tests/support/grading.rs`'s
//! `trivial_baseline()` here — needs a `MeasurementCollection` to carry each
//! graded item's own answer, grouped by family. Until PLAT-1016 no producer
//! did, so `quoin measurement verify`'s `Baseline::ConstantPredictor` arm
//! always answered `constant_predictor_rows_absent`
//! (`quoin_measurement::verify::Reason::ConstantPredictorRowsAbsent`).
//!
//! This module is that producer: [`item_observations`] turns a graded
//! corpus — this crate's own criterion-strength grading, or any caller's
//! `Item` rows — into one [`MeasurementObservation`] per item, under a
//! metric name derived from the governed metric
//! (`quoin_measurement::constant_predictor_item_metric`) so the checker can
//! find them without them being mistaken for another slice of the governed
//! metric itself (`quoin-measurement`'s own run-building filter matches the
//! plan's `metric` exactly).
//!
//! It performs no I/O: like the rest of this crate, writing the resulting
//! observations into a stored `MeasurementCollection` — merging them with the
//! collection's aggregate proportion observation and calling
//! `quoin_measurement::write_measurement_collection` — is a caller's concern
//! (`lib.rs`'s own module doc: the async/blocking and I/O bridges belong to
//! whoever has a CLI entry point, not this library).
//!
//! # Design choice: `dimensions`, not `rawEvidence`
//!
//! quoin FR-108-CON-3 states plainly that `rawEvidence` is not read by the
//! checker: "no schema says what its members mean". Putting the per-item
//! rows there would mean writing a de facto schema for `rawEvidence` that
//! only this one baseline understood, and it would need FR-108-CON-3 to be
//! narrowed just for this feature. `dimensions` (`Record<string, JsonValue>`
//! on every observation, per `crate::types::observation`'s port of
//! `types.ts:22`) already exists for exactly this: distinguishing one slice
//! of a metric from another. A per-item observation is one more slice — of a
//! metric the plan does not itself read, the item metric — identified by
//! `item_id` and carrying `family`, `expected`, `contested` and `actual` as
//! its dimensions. No new population member was needed: `population`
//! describes what was examined in aggregate (`examined`/`matched`/
//! `complete`), and there is nothing aggregate about one item's own label.

use std::collections::BTreeMap;

use quoin_measurement::constant_predictor_dims as dim;
use quoin_measurement::error::{MeasurementError, MeasurementErrorCode};
use quoin_measurement::types::ids::NonEmptyText;
use quoin_measurement::types::observation::{Dimensions, MeasurementShape, MeasurementState};
use quoin_measurement::{MeasurementObservation, constant_predictor_item_metric};
use quoin_store::JsonValue;

/// One graded item: what the corpus recorded, what family it belongs to, and
/// what the graded tool returned for it.
///
/// A minimal, lens-agnostic shape — this module does not read
/// `tests/support/grading.rs`'s `Graded` (test-only, and specific to the
/// criterion-strength corpus's two families); a caller maps its own graded
/// rows onto this instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// This item's identity within the corpus. Must be unique among the
    /// items passed to one [`item_observations`] call — it becomes the
    /// per-item observation's `item_id` dimension, which is how observations
    /// stay distinct under [`crate::types::ids::NonEmptyText`]-checked
    /// `dimensions` equality.
    pub item_id: String,
    /// The answer-space family this item belongs to — the plan's own
    /// Population grouping, not invented here (MP-222).
    pub family: String,
    /// The primary recorded (ground-truth) label.
    pub expected: String,
    /// Every reading the corpus recorded as defensible, [`Self::expected`]
    /// included.
    pub contested: Vec<String>,
    /// What the graded tool returned, for traceability. Not read by the
    /// constant-predictor formula, which depends only on the corpus's own
    /// labels.
    pub actual: String,
}

/// Build one per-item retained observation for each of `items`, so
/// `quoin measurement verify`'s `constant-predictor` baseline arm
/// (`quoin_measurement::verify`) can recompute MP-222/PLAT-932's formula from
/// the stored collection rather than trusting an asserted number.
///
/// `metric` is the plan's own governed metric (the one the aggregate
/// proportion observation carries); the per-item observations are written
/// under `{metric}.constant-predictor-item`
/// (`quoin_measurement::constant_predictor_item_metric`), never under
/// `metric` itself, so they are never read as another slice of the governed
/// measurement.
///
/// # Errors
///
/// [`MeasurementErrorCode::CollectionInvalid`] when `plan_id`,
/// `definition_version` or `metric` is empty, or an item's `item_id`,
/// `family` or `expected` is empty — the identity and label fields this
/// baseline cannot be computed without.
pub fn item_observations(
    plan_id: &str,
    definition_version: &str,
    metric: &str,
    items: &[Item],
) -> Result<Vec<MeasurementObservation>, MeasurementError> {
    let plan_id = text(plan_id, "plan_id")?;
    let definition_version = text(definition_version, "definition_version")?;
    let item_metric_name = constant_predictor_item_metric(metric);
    let item_metric = text(&item_metric_name, "metric")?;
    let unit = text("fraction", "unit")?;

    items
        .iter()
        .map(|item| {
            let mut dimensions: BTreeMap<String, JsonValue> = BTreeMap::new();
            dimensions.insert(
                dim::ITEM_ID.to_owned(),
                JsonValue::string(non_empty(&item.item_id, "item_id")?),
            );
            dimensions.insert(
                dim::FAMILY.to_owned(),
                JsonValue::string(non_empty(&item.family, "family")?),
            );
            dimensions.insert(
                dim::EXPECTED.to_owned(),
                JsonValue::string(non_empty(&item.expected, "expected")?),
            );
            dimensions.insert(
                dim::CONTESTED.to_owned(),
                JsonValue::Array(
                    item.contested
                        .iter()
                        .cloned()
                        .map(JsonValue::string)
                        .collect(),
                ),
            );
            dimensions.insert(
                dim::ACTUAL.to_owned(),
                JsonValue::string(item.actual.clone()),
            );
            // A tool that agreed with the primary reading or a contested
            // alternate scores `1.0`; MP-222's baseline formula never reads
            // this value back (it depends only on `expected`/`contested`),
            // but the field is not left `None` when the state is `Measured` —
            // the value is the observation's own per-item agreement, for
            // anything else that reads this collection later.
            let agreed = item.actual == item.expected || item.contested.contains(&item.actual);
            Ok(MeasurementObservation {
                metric: item_metric.clone(),
                plan_id: plan_id.clone(),
                definition_version: definition_version.clone(),
                state: MeasurementState::Measured,
                value: Some(f64::from(u8::from(agreed))),
                unit: unit.clone(),
                shape: MeasurementShape::Scalar,
                population: None,
                dimensions: Dimensions::stated(dimensions),
                reason: None,
            })
        })
        .collect()
}

/// Parse a non-empty required field, refusing with [`MeasurementErrorCode::CollectionInvalid`].
fn text(value: &str, field: &str) -> Result<NonEmptyText, MeasurementError> {
    NonEmptyText::parse(value, MeasurementErrorCode::CollectionInvalid, field)
}

/// Like [`text`], but returning the owned `String` a dimension value needs
/// rather than the [`NonEmptyText`] a top-level observation field needs.
fn non_empty(value: &str, field: &str) -> Result<String, MeasurementError> {
    if value.is_empty() {
        return Err(MeasurementError::new(
            MeasurementErrorCode::CollectionInvalid,
            format!("requires non-empty `{field}`"),
        ));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use quoin_measurement::error::MeasurementErrorCode;
    use quoin_store::JsonValue;

    use super::{Item, item_observations};

    fn item(item_id: &str, family: &str, expected: &str, contested: &[&str], actual: &str) -> Item {
        Item {
            item_id: item_id.to_owned(),
            family: family.to_owned(),
            expected: expected.to_owned(),
            contested: contested.iter().map(|value| (*value).to_owned()).collect(),
            actual: actual.to_owned(),
        }
    }

    /// One observation per item, under the derived item metric, carrying
    /// every dimension the checker's formula and the traceability contract
    /// both need.
    #[test]
    fn one_observation_per_item_under_the_derived_metric() {
        let items = [
            item("w1", "weakness_kind", "sound", &["sound"], "sound"),
            item("c1", "coverage", "2", &["2", "3"], "1"),
        ];
        let observations = item_observations("MP-1016", "v1", "jev.weakness-kind", &items).unwrap();
        assert_eq!(observations.len(), 2);

        let first = &observations[0];
        assert_eq!(
            first.metric.as_str(),
            "jev.weakness-kind.constant-predictor-item"
        );
        assert_eq!(first.plan_id.as_str(), "MP-1016");
        assert_eq!(first.definition_version.as_str(), "v1");
        assert_eq!(
            first.value,
            Some(1.0),
            "sound agrees with the primary reading"
        );
        let dims = first.dimensions.entries();
        assert_eq!(dims.get("item_id"), Some(&JsonValue::string("w1")));
        assert_eq!(
            dims.get("family"),
            Some(&JsonValue::string("weakness_kind"))
        );
        assert_eq!(dims.get("expected"), Some(&JsonValue::string("sound")));
        assert_eq!(
            dims.get("contested"),
            Some(&JsonValue::Array(vec![JsonValue::string("sound")]))
        );
        assert_eq!(dims.get("actual"), Some(&JsonValue::string("sound")));

        let second = &observations[1];
        assert_eq!(
            second.value,
            Some(0.0),
            "1 is neither the primary nor a contested reading"
        );
    }

    /// An empty `plan_id`, `metric`, or an item's own empty `item_id`/
    /// `family`/`expected` all refuse rather than silently write an
    /// unidentifiable row.
    #[test]
    fn empty_identity_fields_refuse() {
        let items = [item("", "family", "expected", &[], "actual")];
        let error = item_observations("MP-1", "v1", "m", &items).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::CollectionInvalid);

        assert!(item_observations("", "v1", "m", &[]).is_err());
        assert!(item_observations("MP-1", "", "m", &[]).is_err());
    }
}
