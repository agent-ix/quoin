// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The links between records, read once.

use std::collections::BTreeMap;

use crate::records::{Estimate, Phase, Records};

pub(crate) struct Links {
    /// Post-control estimate id to the residual risks that reference it.
    residual_by_estimate: BTreeMap<String, Vec<String>>,
    /// Situation id to the post-control estimates of it.
    post_by_situation: BTreeMap<String, Vec<String>>,
    /// Control id to the `FR` and `NFR` ids that satisfy it.
    pub(crate) requirements: BTreeMap<String, Vec<String>>,
    /// Control id to the ids of the documents that verify it.
    pub(crate) verifications: BTreeMap<String, Vec<String>>,
}

impl Links {
    pub(crate) fn build(records: &Records) -> Self {
        let mut residual_by_estimate: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for risk in records.residual_risks.values() {
            if let Some(estimate) = &risk.estimate {
                residual_by_estimate
                    .entry(estimate.clone())
                    .or_default()
                    .push(risk.id.clone());
            }
        }
        let mut post_by_situation: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for estimate in records.estimates.values() {
            if estimate.phase == Phase::PostControl {
                for situation in &estimate.situations {
                    post_by_situation
                        .entry(situation.clone())
                        .or_default()
                        .push(estimate.id.clone());
                }
            }
        }
        let mut requirements: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut verifications: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for other in records.others.values() {
            if matches!(other.kind.as_str(), "FR" | "NFR") {
                for control in &other.satisfies {
                    requirements
                        .entry(control.clone())
                        .or_default()
                        .push(other.id.clone());
                }
            }
            for control in &other.verifies {
                verifications
                    .entry(control.clone())
                    .or_default()
                    .push(other.id.clone());
            }
        }
        Self {
            residual_by_estimate,
            post_by_situation,
            requirements,
            verifications,
        }
    }

    /// The residual risks that reference the post-control estimate `id`.
    pub(crate) fn residual_risks_of(&self, id: &str) -> &[String] {
        self.residual_by_estimate.get(id).map_or(&[], Vec::as_slice)
    }

    /// The policy ids of an estimate, by FR-146: the policy of the residual
    /// risk on it, or for a pre-control estimate on a post-control estimate of
    /// its situation. Sorted and distinct.
    pub(crate) fn policies_of(&self, records: &Records, estimate: &Estimate) -> Vec<String> {
        let sources: Vec<&str> = match estimate.phase {
            Phase::PostControl => vec![estimate.id.as_str()],
            Phase::PreControl => estimate
                .situations
                .iter()
                .filter_map(|situation| self.post_by_situation.get(situation))
                .flatten()
                .map(String::as_str)
                .collect(),
        };
        let mut policies: Vec<String> = sources
            .into_iter()
            .flat_map(|source| self.residual_risks_of(source))
            .filter_map(|risk| records.residual_risks.get(risk))
            .flat_map(|risk| risk.references.iter().cloned())
            .collect();
        policies.sort();
        policies.dedup();
        policies
    }
}
