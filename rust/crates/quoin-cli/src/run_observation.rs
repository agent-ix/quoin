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
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read run observation {path}: {error}"))?;
        let observation: RunObservationFile = serde_json::from_str(&text)
            .map_err(|error| format!("cannot parse run observation {path}: {error}"))?;
        validate(&observation, path)?;
        if !records.insert(observation.record.clone()) {
            return Err(format!(
                "run observation {path} repeats record {}; each record needs one distinct observation",
                observation.record
            ));
        }
        observations.push(observation);
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
            file.strip_suffix(".json")
                .is_some_and(|stem| !stem.is_empty())
                && !matches!(file, "." | "..")
        })
        && components.next().is_none()
        && !path.contains('\\')
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::read_run_observations;
    use ix_trace_rs::trace;
    use std::fs;

    fn file(value: &serde_json::Value) -> (tempfile::TempDir, String) {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("observation.json");
        fs::write(&path, serde_json::to_vec(value).expect("JSON encoding"))
            .expect("observation file");
        (
            directory,
            path.to_str().expect("UTF-8 temp path").to_owned(),
        )
    }

    fn observation(record: &str) -> serde_json::Value {
        serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": record,
        })
    }

    /// FR-153-AC-12: both read views accept repeatable observation files.
    #[trace("FR-153-AC-12")]
    #[test]
    fn matrix_and_derived_assurance_accept_repeatable_observations() {
        let args = [
            "view",
            "--run-observation",
            "one.json",
            "--run-observation",
            "two.json",
        ];
        for (name, command) in [
            ("matrix", crate::matrix::command()),
            ("assurance", crate::assurance::command()),
        ] {
            let matches = command
                .try_get_matches_from(args.map(|argument| argument.to_owned()))
                .unwrap_or_else(|error| panic!("{name} accepts repeated observations: {error}"));
            assert_eq!(
                matches
                    .get_many::<String>("run-observation")
                    .expect("repeatable option")
                    .count(),
                2
            );
        }
    }

    /// FR-153-AC-17: observation files reject malformed selectors, unknown
    /// members, duplicate record selectors and malformed authority arrays.
    #[trace("FR-153-AC-17")]
    #[test]
    fn malformed_and_duplicate_observation_inputs_are_refused() {
        let (directory, path) = file(&serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": "runs/unit/2026-11-13.json",
            "unknown": true,
        }));
        assert!(read_run_observations(std::slice::from_ref(&path)).is_err());
        drop(directory);

        let (directory, path) = file(&serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": "runs/unit/2026-11-13.json",
            "actor_authority": {
                "actor": "alice",
                "admission_ref": "policy-1",
                "authorized": ["FR-001-AC-1", "FR-001-AC-1"],
            }
        }));
        assert!(read_run_observations(std::slice::from_ref(&path)).is_err());
        drop(directory);

        let (directory, path) = file(&observation("runs/unit/../2026-11-13.json"));
        assert!(read_run_observations(std::slice::from_ref(&path)).is_err());
        drop(directory);

        let (directory, first) = file(&observation("runs/unit/2026-11-13.json"));
        let (directory_two, second) = file(&observation("runs/unit/2026-11-13.json"));
        assert!(read_run_observations(&[first, second]).is_err());
        drop((directory, directory_two));
    }

    /// FR-153-AC-14: individually valid but conflicting selected HEADs are
    /// accepted as observations so the qualifier can report record-conflict.
    #[trace("FR-153-AC-14")]
    #[test]
    fn conflicting_valid_heads_are_retained_for_qualification() {
        let first = serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": "runs/unit/2026-11-13.json",
            "selected_head": "a".repeat(40),
        });
        let second = serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": "runs/scan/2026-11-13.json",
            "selected_head": "b".repeat(40),
        });
        let (directory_one, first) = file(&first);
        let (directory_two, second) = file(&second);
        let inputs = read_run_observations(&[first, second]).expect("typed host facts");
        assert_eq!(inputs.len(), 2);
        drop((directory_one, directory_two));
    }
}
