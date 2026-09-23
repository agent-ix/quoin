// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! MP-222/PLAT-932's constant-predictor baseline: the checker's half
//! (PLAT-1016).
//!
//! Split out of `verify/mod.rs` on responsibility (quoin#464's 700-line
//! ceiling): this is the one baseline kind [`super::baseline`] cannot answer
//! from `history` alone. See this crate's `constant_predictor` producer
//! module in `quoin-jev`, and quoin FR-108's "Constant-predictor baseline"
//! section, for the full producer/checker contract and the shape a run's own
//! item observations carry.

use std::collections::BTreeMap;

use quoin_store::JsonValue;

use crate::types::collection::MeasurementCollection;
use crate::types::observation::{constant_predictor_dims as dim, constant_predictor_item_metric};

/// The size-weighted mean of the best-constant agreement per answer family,
/// computed from `collection`'s own per-item retained observations for
/// `metric`.
///
/// Group the population into the answer-space families the plan's own
/// Population section states (the plan does not invent a grouping here; the
/// producer's own `family` dimension is read as-is). For family `f` with
/// `n_f` items, and for each label `i` recorded as the primary reading or a
/// contested alternate of some item in `f`, let `n_{f,i}` count every item
/// where `i` is the primary reading or a contested alternate (an item with
/// more than one defensible answer counts toward each). That family's best
/// constant score is `max_i(n_{f,i}) / n_f`. The whole population's baseline
/// is the size-weighted mean of the per-family rates: total best-constant
/// agreements over total items.
///
/// Returns `None` when `collection` carries no per-item observations for
/// `metric` under `plan_id`/`definition_version` — the caller reads that as
/// [`super::Reason::ConstantPredictorRowsAbsent`], exactly as before this
/// baseline was implemented, only now also when a collection with an
/// aggregate result simply never retained the rows a constant-predictor rule
/// depends on.
pub(super) fn baseline(
    collection: &MeasurementCollection,
    plan_id: &str,
    definition_version: &str,
    metric: &str,
) -> Option<f64> {
    let item_metric = constant_predictor_item_metric(metric);
    let mut by_family: BTreeMap<String, Vec<(String, Vec<String>)>> = BTreeMap::new();
    for item in &collection.observations {
        if item.plan_id.as_str() != plan_id
            || item.definition_version.as_str() != definition_version
            || item.metric.as_str() != item_metric
        {
            continue;
        }
        let dimensions = item.dimensions.entries();
        let Some(family) = dimensions.get(dim::FAMILY).and_then(JsonValue::as_str) else {
            continue;
        };
        let Some(expected) = dimensions.get(dim::EXPECTED).and_then(JsonValue::as_str) else {
            continue;
        };
        let contested = match dimensions.get(dim::CONTESTED) {
            Some(JsonValue::Array(entries)) => entries
                .iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_owned)
                .collect(),
            _ => Vec::new(),
        };
        by_family
            .entry(family.to_owned())
            .or_default()
            .push((expected.to_owned(), contested));
    }
    if by_family.is_empty() {
        return None;
    }
    let mut hits = 0usize;
    let mut total = 0usize;
    for rows in by_family.values() {
        total += rows.len();
        let mut labels: Vec<&str> = rows.iter().map(|(expected, _)| expected.as_str()).collect();
        labels.sort_unstable();
        labels.dedup();
        let best = labels
            .into_iter()
            .map(|label| {
                rows.iter()
                    .filter(|(expected, contested)| {
                        expected == label || contested.iter().any(|entry| entry == label)
                    })
                    .count()
            })
            .max()
            .unwrap_or(0);
        hits += best;
    }
    if total == 0 {
        return None;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "a graded corpus's item count is far below f64's 2^53 exact-integer bound"
    )]
    let rate = hits as f64 / total as f64;
    Some(rate)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::collections::BTreeMap;

    use quoin_store::{JsonObject, JsonValue};

    use super::baseline;
    use crate::types::collection::MeasurementCollection;
    use crate::types::ids::NonEmptyText;
    use crate::types::observation::{
        Dimensions, MeasurementObservation, MeasurementShape, MeasurementState,
        constant_predictor_dims as dim, constant_predictor_item_metric,
    };

    fn text(value: &str) -> NonEmptyText {
        NonEmptyText::parse(
            value,
            crate::error::MeasurementErrorCode::CollectionInvalid,
            "x",
        )
        .unwrap()
    }

    fn item(
        metric: &str,
        family: &str,
        expected: &str,
        contested: &[&str],
    ) -> MeasurementObservation {
        let mut dimensions: BTreeMap<String, JsonValue> = BTreeMap::new();
        dimensions.insert(dim::FAMILY.to_owned(), JsonValue::string(family));
        dimensions.insert(dim::EXPECTED.to_owned(), JsonValue::string(expected));
        dimensions.insert(
            dim::CONTESTED.to_owned(),
            JsonValue::Array(
                contested
                    .iter()
                    .map(|value| JsonValue::string(*value))
                    .collect(),
            ),
        );
        MeasurementObservation {
            metric: text(&constant_predictor_item_metric(metric)),
            plan_id: text("MP-1"),
            definition_version: text("v1"),
            state: MeasurementState::Measured,
            value: Some(1.0),
            unit: text("fraction"),
            shape: MeasurementShape::Scalar,
            population: None,
            dimensions: Dimensions::stated(dimensions),
            reason: None,
        }
    }

    fn malformed(metric: &str) -> MeasurementObservation {
        // No `family` or `expected` dimension at all.
        MeasurementObservation {
            metric: text(&constant_predictor_item_metric(metric)),
            plan_id: text("MP-1"),
            definition_version: text("v1"),
            state: MeasurementState::Measured,
            value: Some(1.0),
            unit: text("fraction"),
            shape: MeasurementShape::Scalar,
            population: None,
            dimensions: Dimensions::stated(BTreeMap::new()),
            reason: None,
        }
    }

    fn collection(observations: Vec<MeasurementObservation>) -> MeasurementCollection {
        MeasurementCollection {
            schema_version: crate::types::collection::MEASUREMENT_SCHEMA_VERSION,
            collection_id: text("run-1"),
            subject: text("quoin"),
            scope: JsonValue::Object(JsonObject::new()),
            tool_identity: text("test"),
            tool_version: text("0.0.0"),
            config_digest: text("digest-1"),
            timestamp: text("2026-09-23T00:00:00Z"),
            source_revision: text("0000000000000000000000000000000000000000"),
            corpus_revision: None,
            environment: JsonObject::new(),
            verification_stack: None,
            observations,
            raw_evidence: JsonValue::Null,
        }
    }

    /// The two-family worked example from FR-108: `weakness_kind` (9/11 best
    /// constant `sound`) and `coverage` (3/4 best constant `2`) combine to
    /// `(9 + 3) / 15`, not the understated single-global-constant `9/15`.
    #[test]
    fn two_families_combine_as_a_size_weighted_mean() {
        let mut items = Vec::new();
        for (expected, contested) in [
            ("sound", vec!["sound"]),
            ("sound", vec!["sound"]),
            ("sound", vec!["sound"]),
            ("sound", vec!["sound"]),
            ("sound", vec!["sound"]),
            ("sound", vec!["sound"]),
            ("gap", vec!["gap", "sound"]),
            ("gap", vec!["gap", "sound"]),
            ("gap", vec!["gap", "sound"]),
            ("weakness", vec!["weakness"]),
            ("weakness", vec!["weakness"]),
        ] {
            items.push(item("m", "weakness_kind", expected, &contested));
        }
        for (expected, contested) in [
            ("2", vec!["2"]),
            ("2", vec!["2"]),
            ("2", vec!["2"]),
            ("1", vec!["1"]),
        ] {
            items.push(item("m", "coverage", expected, &contested));
        }
        let collection = collection(items);
        let value = baseline(&collection, "MP-1", "v1", "m").unwrap();
        assert!((value - 0.8).abs() < f64::EPSILON, "{value}");
    }

    /// An item observation with no `family`/`expected` dimension is dropped
    /// rather than poisoning the whole computation; the baseline still comes
    /// from the items that are usable.
    #[test]
    fn a_malformed_item_is_dropped_not_fatal() {
        let items = vec![
            item("m", "f", "a", &["a"]),
            item("m", "f", "a", &["a"]),
            item("m", "f", "b", &["b"]),
            malformed("m"),
        ];
        let collection = collection(items);
        let value = baseline(&collection, "MP-1", "v1", "m").unwrap();
        // Only the 3 well-formed items count: best constant "a" agrees twice
        // of three.
        assert!((value - (2.0 / 3.0)).abs() < f64::EPSILON, "{value}");
    }

    /// A collection with only malformed item observations, or none at all,
    /// has no usable rows: `None`, not a baseline of `0`.
    #[test]
    fn no_usable_items_is_none() {
        assert!(baseline(&collection(vec![malformed("m")]), "MP-1", "v1", "m").is_none());
        assert!(baseline(&collection(vec![]), "MP-1", "v1", "m").is_none());
    }

    /// An item observation under a different plan, definition version, or
    /// metric (including the governed metric itself, not its
    /// `.constant-predictor-item` suffix) is not read.
    #[test]
    fn only_the_matching_plan_definition_and_item_metric_are_read() {
        let mut wrong_plan = item("m", "f", "a", &["a"]);
        wrong_plan.plan_id = text("MP-2");
        let mut wrong_definition = item("m", "f", "a", &["a"]);
        wrong_definition.definition_version = text("v2");
        let mut governed_metric = item("m", "f", "a", &["a"]);
        governed_metric.metric = text("m");
        let collection = collection(vec![wrong_plan, wrong_definition, governed_metric]);
        assert!(baseline(&collection, "MP-1", "v1", "m").is_none());
    }
}
