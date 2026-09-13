// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The GitHub-release producer definition.
//!
//! A port of `operational-types.ts:112-136` — the declaration only. The
//! producer that reads one and writes the capability/exercise pair is
//! `github-release-operational.ts`, which lands with quoin#472.

use serde::{Deserialize, Serialize};

use crate::common::identity::{ControlId, EvidencePath};
use crate::common::producer::{Producer, Subject};
use crate::operational::record::{OperationalConfiguration, OperationalScope};

/// What a GitHub release run needs in order to become an operational pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitHubReleaseProducerDefinition {
    /// The prefix the produced record identities carry.
    pub record_prefix: String,
    /// Which workflow file governs the release.
    pub workflow_path: EvidencePath,
    /// Which job in it performs the release.
    pub release_job: String,
    /// Which workflow event the release is accepted from.
    pub accepted_event: String,
    /// The control the produced records are about.
    pub control_id: ControlId,
    /// What the control governs.
    pub subject: Subject,
    /// What produces the records.
    pub producer: Producer,
    /// Where the control is observed.
    pub scope: OperationalScope,
    /// What the control runs under.
    pub configuration: OperationalConfiguration,
    /// The state transition the control supports.
    pub supported_transition: String,
    /// Who may use it.
    pub authorized_roles: Vec<String>,
    /// What it covers.
    pub coverage: String,
    /// What it does not.
    pub limitations: Vec<String>,
    /// Who owns the produced records.
    pub owner: String,
    /// Declared gaps.
    pub gaps: Vec<String>,
    /// Declared actions.
    pub actions: Vec<String>,
    /// How long the control has, in seconds.
    pub clock_deadline_seconds: u64,
    /// Where the retained workflow export is.
    pub workflow_evidence_path: EvidencePath,
    /// Where the retained run export is.
    pub run_evidence_path: EvidencePath,
    /// Where the retained jobs export is.
    pub jobs_evidence_path: EvidencePath,
}
