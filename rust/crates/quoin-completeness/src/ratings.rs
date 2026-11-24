// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Supplier feature-rating completeness facts for validation (PLAT-1636).
//!
//! This module reports authored records as found. It does not interpret a
//! rating's value or derive a score, class, or acceptability decision.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::bundle::BundleDocument;

/// State of the supplier's rating records for one functional requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RatingStatus {
    /// No supplier rating record links to this requirement.
    Unrated,
    /// Exactly one supplier rating record links to this requirement.
    Rated,
    /// More than one supplier rating record links to this requirement.
    RatedMoreThanOnce,
}

/// Authored supplier-rating references for one functional requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureRatingState {
    /// The functional requirement identifier.
    pub feature_id: String,
    /// Whether there are zero, one, or multiple supplier ratings.
    pub status: RatingStatus,
    /// Every supplier rating identifier linked to this requirement.
    pub rating_ids: Vec<String>,
}

/// Report supplier-rating completeness for every FR document in a parsed
/// bundle. Adopter re-ratings are deliberately excluded.
#[must_use]
pub fn assess_feature_ratings(documents: &[BundleDocument]) -> Vec<FeatureRatingState> {
    let mut features = documents
        .iter()
        .filter_map(|document| {
            let id = string_field(&document.frontmatter, "id")?;
            (id.starts_with("FR-") && id[3..].chars().all(|character| character.is_ascii_digit()))
                .then(|| id.to_owned())
        })
        .collect::<Vec<_>>();
    features.sort();
    features.dedup();

    let mut ratings = BTreeMap::<String, Vec<String>>::new();
    for document in documents {
        if string_field(&document.frontmatter, "type") != Some("FeatureRiskRating")
            || string_field(&document.frontmatter, "scope") != Some("supplier")
        {
            continue;
        }
        let Some(rating_id) = string_field(&document.frontmatter, "id") else {
            continue;
        };
        let mut targets = rated_features(&document.frontmatter);
        targets.sort();
        targets.dedup();
        for target in targets {
            if features.binary_search(&target).is_ok() {
                ratings
                    .entry(target)
                    .or_default()
                    .push(rating_id.to_owned());
            }
        }
    }

    features
        .into_iter()
        .map(|feature_id| {
            let mut rating_ids = ratings.remove(&feature_id).unwrap_or_default();
            rating_ids.sort();
            let status = match rating_ids.len() {
                0 => RatingStatus::Unrated,
                1 => RatingStatus::Rated,
                _ => RatingStatus::RatedMoreThanOnce,
            };
            FeatureRatingState {
                feature_id,
                status,
                rating_ids,
            }
        })
        .collect()
}

fn rated_features(frontmatter: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut targets = Vec::new();
    if let Some(relations) = frontmatter.get("relationships").and_then(Value::as_array) {
        for relation in relations {
            if relation.get("type").and_then(Value::as_str) == Some("rates_feature")
                && let Some(target) = relation.get("target").and_then(Value::as_str)
                && let Some(feature) = feature_id(target)
            {
                targets.push(feature.to_owned());
            }
        }
    }
    targets
}

fn feature_id(target: &str) -> Option<&str> {
    let candidate = target.rsplit('/').next().unwrap_or(target);
    (candidate.starts_with("FR-")
        && candidate[3..]
            .chars()
            .all(|character| character.is_ascii_digit()))
    .then_some(candidate)
}

fn string_field<'a>(
    frontmatter: &'a serde_json::Map<String, Value>,
    name: &str,
) -> Option<&'a str> {
    frontmatter.get(name).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleDocument;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn document(id: &str, frontmatter: &Value) -> BundleDocument {
        BundleDocument {
            path: format!("{id}.md"),
            frontmatter: frontmatter.as_object().cloned().unwrap_or_default(),
            body: String::new(),
        }
    }

    fn rating(id: &str, scope: &str, feature: &str) -> BundleDocument {
        document(
            id,
            &json!({
                "id": id,
                "type": "FeatureRiskRating",
                "scope": scope,
                "relationships": [{"target": format!("ix://agent-ix/quoin/{feature}"), "type": "rates_feature"}]
            }),
        )
    }

    #[trace("PLAT-1636-AC-6")]
    #[test]
    fn tc_1636_supplier_rating_states_exclude_adopter_reratings() {
        let docs = vec![
            document("FR-001", &json!({"id":"FR-001", "type":"FR"})),
            document("FR-002", &json!({"id":"FR-002", "type":"FR"})),
            document("FR-003", &json!({"id":"FR-003", "type":"FR"})),
            rating("RR-001", "supplier", "FR-002"),
            rating("RR-002", "supplier", "FR-002"),
            rating("RR-003", "adopter", "FR-001"),
            rating("RR-004", "supplier", "FR-999"),
        ];

        let states = assess_feature_ratings(&docs);
        assert_eq!(states.len(), 3);
        let first = states.first().expect("first feature is present");
        assert_eq!(first.feature_id, "FR-001");
        assert_eq!(first.status, RatingStatus::Unrated);
        assert!(first.rating_ids.is_empty());
        let second = states.get(1).expect("second feature is present");
        assert_eq!(second.feature_id, "FR-002");
        assert_eq!(second.status, RatingStatus::RatedMoreThanOnce);
        assert_eq!(second.rating_ids, ["RR-001", "RR-002"]);
        let third = states.get(2).expect("third feature is present");
        assert_eq!(third.feature_id, "FR-003");
        assert_eq!(third.status, RatingStatus::Unrated);
    }
}
