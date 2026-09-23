// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What a verification is given.
//!
//! Everything a verification reads is **retained evidence**, and none of it is
//! produced here: the record and its parents as they were filed, the
//! attestations and their outputs as they were retained, the decision history
//! as ix-flow wrote it, and the audit reports as FR-032 emitted them. A
//! verification runs no producer and re-audits nothing.
//!
//! The record, its parents, the attestations and the decision history arrive
//! as [`JsonValue`] rather than as typed values, because a verification's job
//! includes deciding that one of them is not valid. A signature that could
//! only accept a valid record could not produce `schema_invalid`.

use engineering_assurance::measurement::{NegativeControls, ProtectedApparatus};
use quoin_store::JsonValue;

/// One selection: which attestation is offered for which proof obligation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    /// The obligation the attestation is offered against.
    pub proof_id: String,
    /// The digest of the attestation offered.
    pub attestation_digest: String,
}

/// One retained attestation and the output filed beside it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedAttestation {
    /// The attestation as retained.
    pub attestation: JsonValue,
    /// The output bytes as retained, or `None` when they are not present.
    pub output: Option<Vec<u8>>,
}

/// One finding from a retained FR-032 audit report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportFinding {
    /// The obligation the finding is about.
    pub obligation: String,
    /// The finding's kind, as the auditor spelled it.
    pub kind: String,
}

/// One obligation the auditor did not evaluate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportUnevaluated {
    /// The obligation left unevaluated.
    pub obligation: String,
}

/// A retained FR-032 audit report.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuditReport {
    /// What the auditor found wrong.
    pub findings: Vec<ReportFinding>,
    /// Which obligations it found discharged.
    pub healthy: Vec<String>,
    /// Which obligations it did not evaluate.
    pub unevaluated: Vec<ReportUnevaluated>,
}

/// A retained audit, with the digest of the report it came from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedAudit {
    /// The obligation the report covers.
    pub proof_id: String,
    /// The report's digest, as the auditor filed it.
    pub report_digest: String,
    /// The report itself.
    pub report: AuditReport,
}

/// The measurement plan a record's objective is linked to, when a
/// verification is asked to check the change against one (PLAT-964).
///
/// Only the plan's id and the two members [`crate::verify::apparatus`] reads
/// are carried here; `quoin-measurement` owns the rest of a
/// `MeasurementPlan`'s shape, and a verification is not a plan load. The two
/// apparatus members are `engineering-assurance`'s own types (its FR-024),
/// read exactly as `quoin-measurement`'s plan intake reads them (PLAT-975) —
/// this crate states no second copy of the entry grammar or the control kinds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoverningPlan {
    /// The `MeasurementPlan` id this plan was resolved from (PLAT-1015,
    /// FR-111), sealed onto the receipt as `governing_plan_id`.
    ///
    /// A member of the plan rather than a sibling of it on
    /// [`VerificationInput`], so a receipt cannot record one plan id (or
    /// none) while its apparatus judgment ran against a different plan (or
    /// one): the id sealed is always the id of the plan that was read.
    pub id: String,
    /// The plan's declared `protected_apparatus`, when it declares one.
    pub protected_apparatus: Option<ProtectedApparatus>,
    /// The plan's declared `negative_controls`, when it declares one.
    pub negative_controls: Option<NegativeControls>,
}

/// Everything one verification reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationInput {
    /// The record under verification, as retained.
    pub record: JsonValue,
    /// Its parent chain, as retained.
    pub parents: Vec<JsonValue>,
    /// The candidate revision the verification is about.
    pub candidate_revision: String,
    /// Which attestation is offered for which obligation.
    pub selections: Vec<Selection>,
    /// The attestations retained, and their outputs.
    pub attestations: Vec<RetainedAttestation>,
    /// The retained ix-flow decision history, or `None` when none was
    /// retained at all.
    pub decision_history: Option<JsonValue>,
    /// The retained audit reports.
    pub audits: Vec<RetainedAudit>,
    /// The repository-relative paths the candidate change's diff touches
    /// (PLAT-964): `/`-separated, as `git diff --name-only` prints them,
    /// compared byte-for-byte against the plan's entries. `None` when no diff
    /// was retained for this verification — distinct from `Some(vec![])`, a
    /// retained diff that touched nothing — so that a plan protecting
    /// apparatus is `diff_missing` rather than vacuously clean. A
    /// verification runs no producer and computes no diff itself, so this is
    /// exactly what the caller retained, like every other member here.
    pub diff_paths: Option<Vec<String>>,
    /// The measurement plan the record's objective is linked to, when the
    /// caller asks this verification to check the change against one
    /// (PLAT-964). `None` when the record supports no plan, or the caller
    /// did not resolve one.
    pub governing_plan: Option<GoverningPlan>,
}
