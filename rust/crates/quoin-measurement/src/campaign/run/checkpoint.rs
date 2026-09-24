// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Write-once completed-attempt checkpoints for same-run-ID resumption.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use engineering_assurance::campaign::{
    CampaignAttempt, CampaignAttemptStatus, CampaignDefinition, MeasurementProcedure,
};
use engineering_assurance::producer_execution::InputBinding;
use serde::{Deserialize, Serialize};

use super::{CampaignRunError, CampaignStoreError, read_typed, run_path};
use crate::campaign::source::{SourceError, VerifiedSource};
use crate::types::plan::MeasurementPlan;

const CHECKPOINT_SCHEMA: &str = "quoin.campaign-checkpoint/v1";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttemptCheckpoint {
    schema: String,
    run_id: String,
    definition_digest: String,
    source_graph_digest: String,
    ordinal: usize,
    attempt: CampaignAttempt,
}

/// Load the contiguous prefix of already retained attempts. A gap rejects
/// before any new invocation; old attempt bytes are never replaced.
pub(super) fn load_prefix(
    repo: &Path,
    run_id: &str,
    definition_digest: &str,
    source_graph_digest: &str,
    total: usize,
) -> Result<Vec<CampaignAttempt>, CampaignRunError> {
    let mut attempts = Vec::new();
    let mut missing = false;
    for ordinal in 1..=total {
        let path = checkpoint_path(repo, run_id, ordinal)?;
        let exists = match fs::symlink_metadata(&path) {
            Ok(_) => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => {
                return Err(CampaignStoreError::Io {
                    path,
                    source: error,
                }
                .into());
            }
        };
        if !exists {
            missing = true;
            continue;
        }
        if missing {
            return Err(CampaignRunError::binding(
                "campaign checkpoint gap".to_owned(),
            ));
        }
        let checkpoint: AttemptCheckpoint = read_typed(&path)?;
        if checkpoint.schema != CHECKPOINT_SCHEMA
            || checkpoint.run_id != run_id
            || checkpoint.definition_digest != definition_digest
            || checkpoint.source_graph_digest != source_graph_digest
            || checkpoint.ordinal != ordinal
        {
            return Err(CampaignRunError::binding(
                "campaign checkpoint identity".to_owned(),
            ));
        }
        attempts.push(checkpoint.attempt);
    }
    Ok(attempts)
}

/// Replay every prior attempt from sealed bytes and exact Git inventories
/// before a dependency selector may read any of its artifacts.
#[allow(
    clippy::too_many_arguments,
    reason = "independent replay requires the complete source-bound campaign context"
)]
pub(super) fn validate_prefix(
    repo: &Path,
    definition: &CampaignDefinition,
    definition_digest: &str,
    run_id: &str,
    attempts: &[CampaignAttempt],
    plans: &[MeasurementPlan],
    procedures: &BTreeMap<String, MeasurementProcedure>,
    sources: &BTreeMap<String, VerifiedSource>,
    plan_source: &VerifiedSource,
) -> Result<(), CampaignRunError> {
    if attempts.is_empty() {
        return Ok(());
    }
    // EA cannot mint a result identity for an invalid request. Its bare
    // preflight claim therefore has no independent authority on restart, even
    // when the checkpoint envelope itself names the expected run and member.
    if attempts
        .iter()
        .any(|attempt| attempt.status == CampaignAttemptStatus::InvalidRequest)
    {
        return Err(CampaignRunError::identity(
            "campaign checkpoint has an unprovable preflight outcome".to_owned(),
        ));
    }
    let source_inputs: BTreeMap<String, Vec<InputBinding>> = sources
        .iter()
        .map(|(name, source)| Ok((name.clone(), source.input_inventory()?)))
        .collect::<Result<_, SourceError>>()?;
    let plan_source_inputs = source_inputs
        .get(&plan_source.repository)
        .ok_or_else(|| CampaignRunError::binding("plan source inventory".to_owned()))?;
    if !crate::campaign::verify::checkpoint_prefix_valid(
        repo,
        definition,
        definition_digest,
        run_id,
        attempts,
        plans,
        procedures,
        &source_inputs,
        plan_source_inputs,
    ) {
        return Err(CampaignRunError::identity(
            "campaign checkpoint evidence".to_owned(),
        ));
    }
    Ok(())
}

/// Publish one completed invocation under a deterministic, write-once path.
pub(super) fn publish(
    repo: &Path,
    run_id: &str,
    definition_digest: &str,
    source_graph_digest: &str,
    ordinal: usize,
    attempt: &CampaignAttempt,
) -> Result<(), CampaignRunError> {
    let path = checkpoint_path(repo, run_id, ordinal)?;
    let checkpoint = AttemptCheckpoint {
        schema: CHECKPOINT_SCHEMA.to_owned(),
        run_id: run_id.to_owned(),
        definition_digest: definition_digest.to_owned(),
        source_graph_digest: source_graph_digest.to_owned(),
        ordinal,
        attempt: attempt.clone(),
    };
    let value = serde_json::to_value(checkpoint)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let parsed = crate::json_bridge::from_serde(&value)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    let bytes = quoin_store::canonical_bytes(&parsed)
        .map_err(|error| CampaignRunError::encoding(error.to_string()))?;
    quoin_store::store::write_content_addressed(&path, &bytes).map_err(|error| {
        CampaignStoreError::Store {
            path,
            message: error.to_string(),
        }
    })?;
    Ok(())
}

fn checkpoint_path(repo: &Path, run_id: &str, ordinal: usize) -> Result<PathBuf, CampaignRunError> {
    run_path(repo, run_id)?;
    if ordinal == 0 {
        return Err(CampaignRunError::binding(
            "campaign checkpoint ordinal".to_owned(),
        ));
    }
    let key = format!("{CHECKPOINT_SCHEMA}\0{run_id}\0{ordinal}");
    let digest = quoin_store::digest_bytes_sha256(key.as_bytes());
    Ok(super::super::store::campaigns_root(repo)
        .join("checkpoints")
        .join(format!("{}.json", digest.as_hex())))
}
