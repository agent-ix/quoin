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

/// An authored supplier rating value for a functional requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureRatingValue {
    /// The identifier of the rating record.
    pub rating_id: String,
    /// The value authored in the rating record.
    pub value: String,
}

/// Rating state and authored values for one functional requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureRatingCoverage {
    /// The requirement's authored rating state and rating record identifiers.
    #[serde(flatten)]
    pub rating: FeatureRatingState,
    /// Values present in the supplier rating records.
    pub rating_values: Vec<FeatureRatingValue>,
}

/// Read-only supplier rating coverage for a specification bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureRatingCoverageReport {
    /// Rating state and values for every functional requirement in the bundle.
    pub features: Vec<FeatureRatingCoverage>,
    /// Spec documents that could not be read while building the report.
    pub unreadable: Vec<crate::bundle::UnreadableDocument>,
}

/// Report supplier-rating completeness for every FR document in a parsed
/// bundle. Adopter re-ratings are deliberately excluded.
#[must_use]
pub fn assess_feature_ratings(
    documents: &[BundleDocument],
    repository_namespace: &str,
) -> Vec<FeatureRatingState> {
    let mut features = documents
        .iter()
        .filter_map(|document| {
            if string_field(&document.frontmatter, "type") != Some("FR") {
                return None;
            }
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
        let mut targets = rated_features(&document.frontmatter, repository_namespace);
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

fn rated_features(
    frontmatter: &serde_json::Map<String, Value>,
    repository_namespace: &str,
) -> Vec<String> {
    let mut targets = Vec::new();
    if let Some(relations) = frontmatter.get("relationships").and_then(Value::as_array) {
        for relation in relations {
            if relation.get("type").and_then(Value::as_str) == Some("rates_feature")
                && let Some(target) = relation.get("target").and_then(Value::as_str)
                && let Some(feature) = feature_id(target, repository_namespace)
            {
                targets.push(feature.to_owned());
            }
        }
    }
    targets
}

fn feature_id<'a>(target: &'a str, repository_namespace: &str) -> Option<&'a str> {
    let target = target.strip_prefix("ix://")?;
    let (namespace, candidate) = target.rsplit_once('/')?;
    if namespace != repository_namespace {
        return None;
    }
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

    fn rating(id: &str, scope: &str, target: &str) -> BundleDocument {
        document(
            id,
            &json!({
                "id": id,
                "type": "FeatureRiskRating",
                "scope": scope,
                "relationships": [{"target": target, "type": "rates_feature"}]
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
            rating("RR-001", "supplier", "ix://agent-ix/quoin/FR-002"),
            rating("RR-002", "supplier", "ix://agent-ix/quoin/FR-002"),
            rating("RR-003", "adopter", "ix://agent-ix/quoin/FR-001"),
            rating("RR-004", "supplier", "ix://agent-ix/quoin/FR-999"),
        ];

        let states = assess_feature_ratings(&docs, "agent-ix/quoin");
        assert_eq!(states.len(), 3);
        assert_eq!(
            states,
            [
                FeatureRatingState {
                    feature_id: "FR-001".to_owned(),
                    status: RatingStatus::Unrated,
                    rating_ids: Vec::new(),
                },
                FeatureRatingState {
                    feature_id: "FR-002".to_owned(),
                    status: RatingStatus::RatedMoreThanOnce,
                    rating_ids: vec!["RR-001".to_owned(), "RR-002".to_owned()],
                },
                FeatureRatingState {
                    feature_id: "FR-003".to_owned(),
                    status: RatingStatus::Unrated,
                    rating_ids: Vec::new(),
                },
            ]
        );
    }

    #[trace("PLAT-1636-AC-6")]
    #[test]
    fn tc_1636_external_rating_uri_cannot_match_local_fr_id() {
        let docs = vec![
            document("FR-001", &json!({"id":"FR-001", "type":"FR"})),
            rating("RR-local", "supplier", "ix://agent-ix/quoin/FR-001"),
            rating(
                "RR-external",
                "supplier",
                "ix://other-org/other-repo/FR-001",
            ),
        ];

        let states = assess_feature_ratings(&docs, "agent-ix/quoin");
        assert_eq!(
            states.first(),
            Some(&FeatureRatingState {
                feature_id: "FR-001".to_owned(),
                status: RatingStatus::Rated,
                rating_ids: vec!["RR-local".to_owned()],
            })
        );
    }
}
