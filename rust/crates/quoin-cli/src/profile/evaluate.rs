// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Evaluating an effective profile through the engine (FR-181, FR-182).
//!
//! Quoin finds the one module that declares each key and hands the set to the
//! engine with the effective context. It never reimplements applicability and
//! states no verdict: the report carries keys, clause ids, outcomes and
//! reasons (NFR-037).

use std::collections::BTreeMap;

use quoin_quire_types::{ClauseBindingOutcome, ClauseBindingReport, ClauseSetKey};
use serde::{Deserialize, Serialize};

use super::model::{EffectiveProfile, Layer, ModuleRef};

/// The engine, as this module needs it. The production implementation links
/// `quoin-quire`; a test can stand in a failing or malformed engine.
pub(crate) trait Engine {
    /// The keys `module` declares.
    fn declared(&self, module: &ModuleRef) -> Result<Vec<ClauseSetKey>, String>;
    /// Whether `module` carries a licence-restricted clause set (FR-137).
    fn is_pack(&self, module: &ModuleRef) -> Result<bool, String>;
    /// The engine's `clause-binding-v1` report for one set, as JSON.
    fn evaluate(
        &self,
        module: &ModuleRef,
        key: &ClauseSetKey,
        context: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, String>;
}

/// Why a run was refused.
#[derive(Debug, thiserror::Error)]
pub(crate) enum EvaluateError {
    /// Two modules declare one key.
    #[error("clause set {key} is declared by both `{first}` and `{second}`")]
    AmbiguousKey {
        /// The key.
        key: String,
        /// The first module.
        first: String,
        /// The second module.
        second: String,
    },
    /// A project-layer key belongs to a licensed pack.
    #[error(
        "clause set {key} in the {layer} layer is declared by licensed pack `{module}`; add it to the user file instead"
    )]
    PackInProject {
        /// The key.
        key: String,
        /// The layer.
        layer: &'static str,
        /// The pack module.
        module: String,
    },
    /// The engine could not load or evaluate a module's set.
    #[error("clause set {key}: {reason}")]
    Engine {
        /// The key.
        key: String,
        /// The engine's reason.
        reason: String,
    },
    /// The engine's report is not `clause-binding-v1`.
    #[error("clause set {key}: the engine report is not clause-binding-v1: {reason}")]
    NotBinding {
        /// The key.
        key: String,
        /// Why validation failed.
        reason: String,
    },
}

impl EvaluateError {
    /// The stable diagnostic code for this refusal.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::AmbiguousKey { .. } => "profile.ambiguous_key",
            Self::PackInProject { .. } => "profile.pack_in_project",
            Self::Engine { .. } => "profile.engine_failed",
            Self::NotBinding { .. } => "profile.report_invalid",
        }
    }
}

/// Whether a listed set was found and evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SetStatus {
    /// A module declares the set and the engine evaluated it.
    Evaluated,
    /// No installed module declares the set.
    Unavailable,
}

/// One listed set in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetEntry {
    /// The exact key.
    pub(crate) key: ClauseSetKey,
    /// The file layer that listed it.
    pub(crate) layer: Layer,
    /// Evaluated or unavailable.
    pub(crate) status: SetStatus,
    /// The engine report, when evaluated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) report: Option<ClauseBindingReport>,
}

/// Clause counts per outcome, summed over evaluated sets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClauseCounts {
    /// Clauses that apply.
    pub(crate) binding: usize,
    /// Clauses that do not apply.
    pub(crate) not_binding: usize,
    /// Clauses the context cannot decide.
    pub(crate) unresolved: usize,
}

/// The report's summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Summary {
    /// Sets the engine evaluated.
    pub(crate) sets_evaluated: usize,
    /// Sets no module declares.
    pub(crate) sets_unavailable: usize,
    /// Clause counts.
    pub(crate) clauses: ClauseCounts,
}

/// The one accepted `schemaVersion` of the evaluation report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum EvaluationSchema {
    /// `source-profile-evaluation-v1`.
    #[serde(rename = "source-profile-evaluation-v1")]
    V1,
}

/// The `source-profile-evaluation-v1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Evaluation {
    /// Always `source-profile-evaluation-v1`.
    pub(crate) schema_version: EvaluationSchema,
    /// The profile name.
    pub(crate) profile: String,
    /// The effective context.
    pub(crate) context: BTreeMap<String, String>,
    /// The listed sets, in profile order.
    pub(crate) sets: Vec<SetEntry>,
    /// Counts.
    pub(crate) summary: Summary,
}

fn key_text(key: &ClauseSetKey) -> String {
    format!("{}/{}@{}", key.authority, key.id, key.version)
}

fn layer_name(layer: Layer) -> &'static str {
    match layer {
        Layer::Profile => "profile",
        Layer::Project => "project",
        Layer::User => "user",
    }
}

/// Validate an engine report against `clause-binding-v1`.
fn validate(
    key: &ClauseSetKey,
    report: serde_json::Value,
) -> Result<ClauseBindingReport, EvaluateError> {
    serde_json::from_value(report).map_err(|e| EvaluateError::NotBinding {
        key: key_text(key),
        reason: e.to_string(),
    })
}

/// Evaluate every set of `profile` against its context.
pub(crate) fn evaluate(
    profile: &EffectiveProfile,
    modules: &[ModuleRef],
    engine: &dyn Engine,
) -> Result<Evaluation, EvaluateError> {
    let mut declared: Vec<(&ModuleRef, Vec<ClauseSetKey>)> = Vec::new();
    for module in modules {
        let keys = engine
            .declared(module)
            .map_err(|reason| EvaluateError::Engine {
                key: format!("module `{}`", module.name),
                reason,
            })?;
        declared.push((module, keys));
    }

    let mut sets = Vec::with_capacity(profile.sets.len());
    for (key, layer) in &profile.sets {
        let mut owners = declared
            .iter()
            .filter(|(_, keys)| keys.contains(key))
            .map(|(module, _)| *module);
        let (Some(owner), second) = (owners.next(), owners.next()) else {
            sets.push(SetEntry {
                key: key.clone(),
                layer: *layer,
                status: SetStatus::Unavailable,
                report: None,
            });
            continue;
        };
        if let Some(second) = second {
            return Err(EvaluateError::AmbiguousKey {
                key: key_text(key),
                first: owner.name.clone(),
                second: second.name.clone(),
            });
        }
        if *layer == Layer::Project
            && engine
                .is_pack(owner)
                .map_err(|reason| EvaluateError::Engine {
                    key: key_text(key),
                    reason,
                })?
        {
            return Err(EvaluateError::PackInProject {
                key: key_text(key),
                layer: layer_name(*layer),
                module: owner.name.clone(),
            });
        }
        let raw = engine
            .evaluate(owner, key, &profile.context)
            .map_err(|reason| EvaluateError::Engine {
                key: key_text(key),
                reason,
            })?;
        sets.push(SetEntry {
            key: key.clone(),
            layer: *layer,
            status: SetStatus::Evaluated,
            report: Some(validate(key, raw)?),
        });
    }

    let mut summary = Summary {
        sets_evaluated: 0,
        sets_unavailable: 0,
        clauses: ClauseCounts::default(),
    };
    for entry in &sets {
        let Some(report) = &entry.report else {
            summary.sets_unavailable += 1;
            continue;
        };
        summary.sets_evaluated += 1;
        for clause in &report.clauses {
            match clause.outcome {
                ClauseBindingOutcome::Binding => summary.clauses.binding += 1,
                ClauseBindingOutcome::NotBinding => summary.clauses.not_binding += 1,
                ClauseBindingOutcome::Unresolved => summary.clauses.unresolved += 1,
            }
        }
    }
    Ok(Evaluation {
        schema_version: EvaluationSchema::V1,
        profile: profile.name.clone(),
        context: profile.context.clone(),
        sets,
        summary,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests;
