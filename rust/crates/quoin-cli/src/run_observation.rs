// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Read-time host observations for retained analysis runs (FR-153).

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;

/// One caller-supplied observation file. Signature contents remain opaque
/// until the host verifies them against the history-derived introducing commit.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunObservationFile {
    /// The input protocol.
    pub(crate) protocol: String,
    /// Exact canonical store-relative run record path.
    pub(crate) record: String,
    /// Source HEAD selected for this read operation.
    pub(crate) selected_head: Option<String>,
    /// Checkout SHA observed for the run-package execution.
    pub(crate) checkout_sha: Option<String>,
    /// CI execution identity.
    pub(crate) ci_association: Option<CiAssociation>,
    /// Complete signature verification result, not yet admitted.
    pub(crate) signature_result: Option<Value>,
    /// Claimed actor authority, not yet admitted.
    pub(crate) actor_authority: Option<ActorAuthority>,
}

/// Exact CI association members required by FR-153.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CiAssociation {
    /// CI system.
    pub(crate) system: String,
    /// Project or repository.
    pub(crate) project: String,
    /// Run identifier.
    pub(crate) run: String,
    /// Attempt identifier.
    pub(crate) attempt: String,
    /// Job identifier.
    pub(crate) job: String,
    /// Invocation identifier.
    pub(crate) invocation: String,
}

/// Claimed actor scope; identity is admitted only after verified signature evidence.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActorAuthority {
    /// Claimed actor principal.
    pub(crate) actor: String,
    /// Admission reference.
    pub(crate) admission_ref: String,
    /// Obligations this actor may assert.
    pub(crate) authorized: Vec<String>,
}

/// Load and validate repeated `--run-observation` inputs.
pub(crate) fn read_run_observations(paths: &[String]) -> Result<Vec<RunObservationFile>, String> {
    let mut observations = Vec::new();
    let mut records = BTreeSet::new();
    for path in paths {
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read run observation {path}: {error}"))?;
        let observation: RunObservationFile = serde_json::from_str(&text)
            .map_err(|error| format!("cannot parse run observation {path}: {error}"))?;
        validate(&observation, &path)?;
        if !records.insert(observation.record.clone()) {
            return Err(format!(
                "run observation {path} repeats record {}; each record needs one distinct observation",
                observation.record
            ));
        }
        observations.push(observation);
    }
    if let Some(selected_head) = observations
        .iter()
        .filter_map(|observation| observation.selected_head.as_deref())
        .filter(|head| is_object_id(head))
        .find(|head| is_object_id(head))
    {
        if observations
            .iter()
            .filter_map(|observation| observation.selected_head.as_deref())
            .filter(|head| is_object_id(head))
            .any(|head| head != selected_head)
        {
            return Err("run observations supply different valid selected_head values".to_owned());
        }
    }
    Ok(observations)
}

fn validate(observation: &RunObservationFile, path: &str) -> Result<(), String> {
    if observation.protocol != "quoin.analysis-run-observation/v1" {
        return Err(format!(
            "run observation {path} has protocol {:?}, expected quoin.analysis-run-observation/v1",
            observation.protocol
        ));
    }
    if observation.record.is_empty() || !canonical_run_path(&observation.record) {
        return Err(format!(
            "run observation {path} record must be a canonical runs/<suite>/<timestamp>.json path"
        ));
    }
    if observation
        .signature_result
        .as_ref()
        .is_some_and(|result| !result.is_object())
    {
        return Err(format!(
            "run observation {path} signature_result must be an object"
        ));
    }
    if let Some(ci) = &observation.ci_association {
        for (member, value) in [
            ("system", &ci.system),
            ("project", &ci.project),
            ("run", &ci.run),
            ("attempt", &ci.attempt),
            ("job", &ci.job),
            ("invocation", &ci.invocation),
        ] {
            if value.is_empty() {
                return Err(format!(
                    "run observation {path} ci_association.{member} is empty"
                ));
            }
        }
    }
    if let Some(actor) = &observation.actor_authority {
        if actor.actor.is_empty() || actor.admission_ref.is_empty() {
            return Err(format!(
                "run observation {path} actor_authority actor and admission_ref must be non-empty"
            ));
        }
        let mut authorized = BTreeSet::new();
        if actor
            .authorized
            .iter()
            .any(|id| id.is_empty() || !authorized.insert(id))
        {
            return Err(format!(
                "run observation {path} actor_authority.authorized contains an empty or duplicate id"
            ));
        }
    }
    Ok(())
}

fn canonical_run_path(path: &str) -> bool {
    let mut components = path.split('/');
    matches!(components.next(), Some("runs"))
        && components
            .next()
            .is_some_and(|suite| !suite.is_empty() && suite != "." && suite != "..")
        && components.next().is_some_and(|file| {
            file.ends_with(".json") && file.len() > 5 && !matches!(file, "." | "..")
        })
        && components.next().is_none()
        && !path.contains('\\')
}

fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
