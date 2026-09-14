// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `auditor` domain's wire shapes, and the ceilings they are read under.
//!
//! Request and payload types only. The operations that take them live in
//! [`super`], and the exit-taxonomy mapping in [`super::taxonomy`].
//!
//! # Why these requests are large and the `graph` domain's are not
//!
//! `graph.*` names four paths and a granted reader opens them. Nothing here
//! opens anything: `quoin_auditor` is [`Inert`](quoin_auditor::Inert), so the
//! whole evidence store — bindings, runs, scans, injections — rides on stdin as
//! data the command already read. That is the property that makes the auditor
//! auditable, so the ceiling is sized for a store rather than for a filename.

use std::collections::BTreeMap;

use quoin_auditor::advise::PropertyShape;
use quoin_auditor::{Advice, AuditInput, MethodCatalog};
use quoin_evidence::types::{Binding, IndependenceAssessment, RunRecord};
use quoin_finding_types::{AuditReport, Finding};
use quoin_quire_types::{CoverageDiagnostic, Obligation};
use serde::{Deserialize, Serialize};

/// The largest `auditor.*` request this domain will decide over, in bytes.
///
/// 32 MiB: half the transport ceiling, and four times the largest bound any
/// other domain declares. A request here is a whole evidence store rather than
/// a filename, so it is sized for the thing that grew — the payload that hit
/// `maxBuffer` at 1,090,714 bytes (agent-ix/quoin#164) is 3% of it — while
/// still leaving the transport the headroom
/// `tc_412_the_transport_ceiling_stays_above_every_domain_bound` requires.
pub const MAX_AUDITOR_REQUEST_BYTES: usize = 32 * 1024 * 1024;

/// What `quoin assurance` and `quoin evidence audit` send.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AuditRequest {
    /// Everything the audit reads, assembled by the caller.
    pub input: AuditInput,
    /// The accepted-finding keys a `--ratchet` run was given.
    ///
    /// `None` is not `Some([])`. Absent means no baseline was read and the
    /// full report is the answer; an empty baseline means one was read and
    /// accepted nothing, so every finding is new. Labelling the first case
    /// "new violations only" told a day-one reader their whole backlog was new
    /// (agent-ix/quoin#169), so the two stay distinguishable on the wire.
    #[serde(default)]
    pub accepted: Option<Vec<String>>,
}

/// What an audit answered.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AuditPayload {
    /// The whole report: findings, healthy, unevaluated.
    pub report: AuditReport,
    /// Profile-selected evidence independence, when a policy was given.
    ///
    /// # Why this is lifted out of the report
    ///
    /// `quoin-auditor` writes it into [`AuditReport`]'s passthrough map, and
    /// that ruling is about READING retained store bytes wider than the
    /// declared type (quoin#383/#385): a captured corpus holds an
    /// `independence` member that is not an assessment at all, so the retained
    /// type cannot declare one.
    ///
    /// The boundary is not reading retained bytes. It is answering a request
    /// it just computed, and it knows exactly what it put there — so the
    /// member is MOVED here and typed, rather than copied. Copied would be the
    /// same list encoded twice, the shape FR-097 forbids, and left in place it
    /// would reach TypeScript as an undeclared key no generated type carries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub independence: Option<Vec<IndependenceAssessment>>,
    /// The findings that survived the ratchet, when one was applied.
    ///
    /// `None` when the request carried no baseline — the caller then reports
    /// `report.findings`. Present-and-equal would be the same list encoded
    /// twice, which is the shape FR-097 forbids.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported: Option<Vec<Finding>>,
}

/// What `quoin evidence baseline` sends: the same audit, a different question.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BaselineRequest {
    /// Everything the audit reads, assembled by the caller.
    pub input: AuditInput,
}

/// The keys a baseline would accept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BaselinePayload {
    /// Every finding's key, sorted, as the baseline file records them.
    pub accepted: Vec<String>,
}

/// What `quoin advise` sends: one request for every obligation, not one each.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdviseRequest {
    /// The merged verification-method catalog.
    pub catalog: MethodCatalog,
    /// The obligations the coverage payload carried, in its order.
    pub obligations: Vec<Obligation>,
    /// Obligation id → what `quire properties` classified the criterion as.
    ///
    /// A second engine call the caller makes, because the coverage payload
    /// carries neither field and this domain spawns nothing.
    #[serde(default)]
    pub shapes: BTreeMap<String, PropertyShape>,
    /// The binding graph from the store.
    #[serde(default)]
    pub bindings: Vec<Binding>,
    /// Every run record the store holds.
    #[serde(default)]
    pub runs: Vec<RunRecord>,
    /// The coverage payload's diagnostics, for the uncatalogued-method join.
    #[serde(default)]
    pub diagnostics: Vec<CoverageDiagnostic>,
}

/// One advice row per obligation, plus what the vocabulary join could tell.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AdvisePayload {
    /// Advice, in the order the obligations arrived.
    pub advice: Vec<Advice>,
    /// True when the engine emitted the diagnostic without a `value`.
    ///
    /// The caller warns on it. It is carried separately from the advice
    /// because it is a fact about the ENGINE, not about any obligation: every
    /// row still got an answer, and the answer is two-state rather than three.
    pub degraded: bool,
}
