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

use std::collections::{BTreeMap, BTreeSet};

use quoin_store::JsonValue;

use super::Reason;
use crate::types::collection::MeasurementCollection;
use crate::types::observation::{
    MeasurementObservation, constant_predictor_dims as dim, constant_predictor_item_metric,
};

/// One usable item row: its primary reading and its contested alternates.
struct Row<'a> {
    expected: &'a str,
    contested: Vec<&'a str>,
}

impl Row<'_> {
    /// Whether a constant answer of `label` agrees with this item.
    fn agrees(&self, label: &str) -> bool {
        self.expected == label || self.contested.contains(&label)
    }
}

/// The size-weighted mean of the best-constant agreement per answer family,
/// computed from `collection`'s own per-item retained observations for
/// `observation`'s metric, plan and definition.
///
/// Group the population into the answer-space families the plan's own
/// Population section states (the plan does not invent a grouping here; the
/// producer's own `family` dimension is read as-is). For family `f` with
/// `n_f` items, and for each label `i` in that family's answer space — every
/// label recorded as some item's primary reading *or* a contested alternate
/// — let `n_{f,i}` count every item where `i` is the primary reading or a
/// contested alternate (an item with more than one defensible answer counts
/// toward each). That family's best constant score is `max_i(n_{f,i}) / n_f`.
/// The whole population's baseline is the size-weighted mean of the
/// per-family rates: total best-constant agreements over total items.
///
/// # Errors
///
/// Fails closed rather than computing a baseline from a sample it cannot
/// vouch for:
///
/// - [`Reason::ConstantPredictorRowsAbsent`] when the collection carries no
///   item observation for this metric, plan and definition.
/// - [`Reason::ConstantPredictorRowsMalformed`] when any such item
///   observation lacks a non-empty string `item_id`, `family` or `expected`,
///   carries a `contested` that is not an array of strings, or repeats
///   another item's `item_id`. A dropped row would shrink the sample the
///   baseline is computed from without anyone noticing.
/// - [`Reason::ConstantPredictorRowsMismatch`] when the number of item
///   observations is not `observation`'s own `population.examined`: the item
///   rows are not the population the governed rate was measured over (items
///   lost or duplicated by the producer, or a sliced observation whose slice
///   the population-wide item rows do not describe).
pub(super) fn baseline(
    collection: &MeasurementCollection,
    observation: &MeasurementObservation,
) -> Result<f64, Reason> {
    let item_metric = constant_predictor_item_metric(observation.metric.as_str());
    let mut by_family: BTreeMap<&str, Vec<Row<'_>>> = BTreeMap::new();
    let mut item_ids: BTreeSet<&str> = BTreeSet::new();
    let mut total = 0usize;
    for item in &collection.observations {
        if item.plan_id != observation.plan_id
            || item.definition_version != observation.definition_version
            || item.metric.as_str() != item_metric
        {
            continue;
        }
        let dimensions = item.dimensions.entries();
        let text = |key: &str| {
            dimensions
                .get(key)
                .and_then(JsonValue::as_str)
                .filter(|value| !value.is_empty())
        };
        let (Some(item_id), Some(family), Some(expected)) =
            (text(dim::ITEM_ID), text(dim::FAMILY), text(dim::EXPECTED))
        else {
            return Err(Reason::ConstantPredictorRowsMalformed);
        };
        let Some(JsonValue::Array(entries)) = dimensions.get(dim::CONTESTED) else {
            return Err(Reason::ConstantPredictorRowsMalformed);
        };
        let contested = entries
            .iter()
            .map(JsonValue::as_str)
            .collect::<Option<Vec<&str>>>()
            .ok_or(Reason::ConstantPredictorRowsMalformed)?;
        if !item_ids.insert(item_id) {
            return Err(Reason::ConstantPredictorRowsMalformed);
        }
        total += 1;
        by_family.entry(family).or_default().push(Row {
            expected,
            contested,
        });
    }
    if total == 0 {
        return Err(Reason::ConstantPredictorRowsAbsent);
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "a graded corpus's item count is far below f64's 2^53 exact-integer bound"
    )]
    let total_f64 = total as f64;
    let examined = observation
        .population
        .as_ref()
        .and_then(|population| population.examined);
    #[allow(
        clippy::float_cmp,
        reason = "both sides are whole-number counts, exact in f64 at any corpus size"
    )]
    let consistent = examined == Some(total_f64);
    if !consistent {
        return Err(Reason::ConstantPredictorRowsMismatch);
    }
    let hits: usize = by_family.values().map(|rows| best_constant(rows)).sum();
    #[allow(
        clippy::cast_precision_loss,
        reason = "a graded corpus's item count is far below f64's 2^53 exact-integer bound"
    )]
    let rate = hits as f64 / total_f64;
    Ok(rate)
}

/// One family's best-constant hit count: the most items any single label in
/// the family's answer space (every primary reading and contested alternate)
/// agrees with.
fn best_constant(rows: &[Row<'_>]) -> usize {
    let labels: BTreeSet<&str> = rows
        .iter()
        .flat_map(|row| std::iter::once(row.expected).chain(row.contested.iter().copied()))
        .collect();
    labels
        .into_iter()
        .map(|label| rows.iter().filter(|row| row.agrees(label)).count())
        .max()
        .unwrap_or(0)
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
        Dimensions, MeasurementObservation, MeasurementPopulation, MeasurementShape,
        MeasurementState, constant_predictor_dims as dim, constant_predictor_item_metric,
    };
    use crate::verify::Reason;

    fn text(value: &str) -> NonEmptyText {
        NonEmptyText::parse(
            value,
            crate::error::MeasurementErrorCode::CollectionInvalid,
            "x",
        )
        .unwrap()
    }

    fn row(
        item_id: &str,
        family: &str,
        expected: &str,
        contested: &[&str],
    ) -> BTreeMap<String, JsonValue> {
        let mut dimensions: BTreeMap<String, JsonValue> = BTreeMap::new();
        dimensions.insert(dim::ITEM_ID.to_owned(), JsonValue::string(item_id));
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
        dimensions
    }

    fn observation(
        metric: &str,
        dimensions: BTreeMap<String, JsonValue>,
    ) -> MeasurementObservation {
        MeasurementObservation {
            metric: text(metric),
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

    fn item(
        item_id: &str,
        family: &str,
        expected: &str,
        contested: &[&str],
    ) -> MeasurementObservation {
        observation(
            &constant_predictor_item_metric("m"),
            row(item_id, family, expected, contested),
        )
    }

    /// The governed aggregate observation, examining `examined` items.
    fn governed(examined: f64) -> MeasurementObservation {
        let mut aggregate = observation("m", BTreeMap::new());
        aggregate.dimensions = Dimensions::ABSENT;
        aggregate.population = Some(MeasurementPopulation {
            examined: Some(examined),
            matched: Some(0.0),
            complete: Some(true),
            repetitions: None,
            identity: None,
            unmodelled: BTreeMap::new(),
        });
        aggregate
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

    /// FR-108's worked example: `weakness_kind` (11 items; `sound` agrees
    /// with 6 primary readings and 3 contested alternates, 9/11) and
    /// `coverage` (4 items; level `2`, 3/4) combine to `(9 + 3) / 15 = 0.8`,
    /// not the understated single-global-constant `9/15 = 0.6`.
    #[test]
    fn two_families_combine_as_a_size_weighted_mean() {
        let mut items = Vec::new();
        let weakness_kind: [(&str, &[&str]); 11] = [
            ("sound", &["sound"]),
            ("sound", &["sound"]),
            ("sound", &["sound"]),
            ("sound", &["sound"]),
            ("sound", &["sound"]),
            ("sound", &["sound"]),
            ("gap", &["gap", "sound"]),
            ("gap", &["gap", "sound"]),
            ("gap", &["gap", "sound"]),
            ("weakness", &["weakness"]),
            ("weakness", &["weakness"]),
        ];
        for (index, (expected, contested)) in weakness_kind.into_iter().enumerate() {
            items.push(item(
                &format!("w{index}"),
                "weakness_kind",
                expected,
                contested,
            ));
        }
        let coverage: [(&str, &[&str]); 4] =
            [("2", &["2"]), ("2", &["2"]), ("2", &["2"]), ("1", &["1"])];
        for (index, (expected, contested)) in coverage.into_iter().enumerate() {
            items.push(item(&format!("c{index}"), "coverage", expected, contested));
        }
        let value = baseline(&collection(items), &governed(15.0)).unwrap();
        assert!((value - 0.8).abs() < f64::EPSILON, "{value}");
    }

    /// A label recorded only as a contested alternate, never as any item's
    /// primary reading, is still in the family's answer space: here `x`
    /// agrees with all three items, so the baseline is `1.0`, not the `1/3`
    /// a primary-readings-only candidate set would understate it as.
    #[test]
    fn a_contested_only_label_is_a_candidate_constant() {
        let items = vec![
            item("a", "f", "p", &["p", "x"]),
            item("b", "f", "q", &["q", "x"]),
            item("c", "f", "r", &["r", "x"]),
        ];
        let value = baseline(&collection(items), &governed(3.0)).unwrap();
        assert!((value - 1.0).abs() < f64::EPSILON, "{value}");
    }

    /// Any malformed item observation fails the baseline closed rather than
    /// being dropped: a dropped row shrinks the sample silently.
    #[test]
    fn a_malformed_item_fails_closed() {
        let well_formed = || {
            vec![
                item("a", "f", "a", &["a"]),
                item("b", "f", "a", &["a"]),
                item("c", "f", "b", &["b"]),
            ]
        };
        let metric = constant_predictor_item_metric("m");
        let mut no_family = row("d", "f", "a", &["a"]);
        no_family.remove(dim::FAMILY);
        let mut empty_expected = row("d", "f", "a", &["a"]);
        empty_expected.insert(dim::EXPECTED.to_owned(), JsonValue::string(""));
        let mut no_item_id = row("d", "f", "a", &["a"]);
        no_item_id.remove(dim::ITEM_ID);
        let mut contested_not_array = row("d", "f", "a", &["a"]);
        contested_not_array.insert(dim::CONTESTED.to_owned(), JsonValue::string("a"));
        let mut contested_not_strings = row("d", "f", "a", &["a"]);
        contested_not_strings.insert(
            dim::CONTESTED.to_owned(),
            JsonValue::Array(vec![JsonValue::Bool(true)]),
        );
        let duplicate_id = row("a", "g", "a", &["a"]);
        for bad in [
            no_family,
            empty_expected,
            no_item_id,
            contested_not_array,
            contested_not_strings,
            duplicate_id,
        ] {
            let mut items = well_formed();
            items.push(observation(&metric, bad.clone()));
            assert_eq!(
                baseline(&collection(items), &governed(4.0)),
                Err(Reason::ConstantPredictorRowsMalformed),
                "{bad:?}"
            );
        }
    }

    /// Item rows that do not number the governed observation's own
    /// `examined` are not its population: lost rows, extra rows, and an
    /// aggregate with no stated `examined` all fail closed.
    #[test]
    fn an_item_count_other_than_examined_fails_closed() {
        let items = || {
            vec![
                item("a", "f", "a", &["a"]),
                item("b", "f", "a", &["a"]),
                item("c", "f", "b", &["b"]),
            ]
        };
        for examined in [2.0, 4.0, 30.0] {
            assert_eq!(
                baseline(&collection(items()), &governed(examined)),
                Err(Reason::ConstantPredictorRowsMismatch),
                "{examined}"
            );
        }
        let mut unstated = governed(3.0);
        unstated.population = None;
        assert_eq!(
            baseline(&collection(items()), &unstated),
            Err(Reason::ConstantPredictorRowsMismatch)
        );
        let value = baseline(&collection(items()), &governed(3.0)).unwrap();
        assert!((value - (2.0 / 3.0)).abs() < f64::EPSILON, "{value}");
    }

    /// A collection with no item observation for the metric is absent, not
    /// a baseline of `0`; nor is one under a different plan, definition
    /// version, or metric (including the governed metric itself) read.
    #[test]
    fn only_the_matching_plan_definition_and_item_metric_are_read() {
        assert_eq!(
            baseline(&collection(vec![]), &governed(1.0)),
            Err(Reason::ConstantPredictorRowsAbsent)
        );
        let mut wrong_plan = item("a", "f", "a", &["a"]);
        wrong_plan.plan_id = text("MP-2");
        let mut wrong_definition = item("b", "f", "a", &["a"]);
        wrong_definition.definition_version = text("v2");
        let mut governed_metric = item("c", "f", "a", &["a"]);
        governed_metric.metric = text("m");
        let mut other_metric = item("d", "f", "a", &["a"]);
        other_metric.metric = text(&constant_predictor_item_metric("n"));
        let collection = collection(vec![
            wrong_plan,
            wrong_definition,
            governed_metric,
            other_metric,
        ]);
        assert_eq!(
            baseline(&collection, &governed(4.0)),
            Err(Reason::ConstantPredictorRowsAbsent)
        );
    }
}
