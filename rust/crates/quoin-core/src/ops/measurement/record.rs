// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `measurement.record`: one route, three intakes.
//!
//! The retained `src/commands/measurement/record.ts` reads a document, looks at
//! its `record_type` member and calls one of three publishing functions. That
//! branch is the operation, so it is what crosses the boundary: one route, and
//! the document's own member decides which intake accepts it.
//!
//! **Route coverage is not handler coverage (quoin#447).** Three intakes behind
//! one route is three handlers, and a test that exercises the route once leaves
//! two of them never executed. They are distinguished by the path each returns —
//! `<store>/measurements/<id>.json`, `<store>/interventions/p-<id>.json` and
//! `<store>/operational/<id>.json` — and `tests/tc_478_measurement_dispatch.rs`
//! drives the real binary once per intake and asserts on that path.

use std::path::Path;

use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::operational::intake::write_operational_record;
use quoin_measurement::source::SystemClock;
use quoin_measurement::{write_intervention_record, write_measurement_collection};

use crate::error::CoreError;
use crate::protocol::Response;

use super::taxonomy::{map_intake, map_measurement};
use super::wire::{
    MAX_COLLECTION_BYTES, MAX_INTERVENTION_RECORD_BYTES, MAX_OPERATIONAL_RECORD_BYTES,
    RecordRequest,
};
use super::{bound, parse, path_payload};

/// Which intake a candidate document belongs to.
///
/// A parsed value rather than a `bool` or a `&str` compared three times
/// (rust-style §7, parse don't validate): the member is read ONCE, and every
/// later question — which ceiling applies, which intake runs — is asked of this
/// type, so the three answers cannot disagree about what the document is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Intake {
    /// A measurement collection. The default: `record.ts` treats every
    /// `record_type` it does not recognise, and an absent one, as a collection.
    Collection,
    /// An `intervention_experiment` record.
    Intervention,
    /// An `operational_evidence` record.
    Operational,
}

impl Intake {
    /// The intake a candidate's own `record_type` member names.
    pub(super) fn of(record: &serde_json::Value) -> Self {
        match record
            .get("record_type")
            .and_then(serde_json::Value::as_str)
        {
            Some("intervention_experiment") => Self::Intervention,
            Some("operational_evidence") => Self::Operational,
            _ => Self::Collection,
        }
    }

    /// The ceiling this intake's request is read under.
    pub(super) const fn limit(self) -> usize {
        match self {
            Self::Collection => MAX_COLLECTION_BYTES,
            Self::Intervention => MAX_INTERVENTION_RECORD_BYTES,
            Self::Operational => MAX_OPERATIONAL_RECORD_BYTES,
        }
    }
}

/// Answer a `measurement.record`.
///
/// The ceiling is applied to the WHOLE request before any store is opened, and
/// which ceiling applies is decided by reading one member of the already-parsed
/// document — no file is touched to find out.
///
/// # Errors
///
/// - [`crate::error::CoreErrorCode::BadRequest`] when stdin is not a
///   [`RecordRequest`], or when the candidate itself is malformed.
/// - [`crate::error::CoreErrorCode::Refused`] when the request exceeds the
///   intake's ceiling, or when the store declines to publish.
pub fn record(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.record";
    let intake = request.get("record").map_or(Intake::Collection, Intake::of);
    bound(request, OP, intake.limit())?;

    let request: RecordRequest = parse(request, OP)?;
    let repo = Path::new(&request.repo);
    let path = match intake {
        Intake::Collection => {
            let stored = from_serde(&request.record).map_err(|e| map_measurement(&e, OP))?;
            write_measurement_collection(repo, &stored).map_err(|e| map_measurement(&e, OP))?
        }
        Intake::Intervention => {
            let stored = from_serde(&request.record).map_err(|e| map_measurement(&e, OP))?;
            write_intervention_record(repo, &stored).map_err(|e| map_intake(&e, OP))?
        }
        // The clock is the store's own seam, and the one this route needs: the
        // operational intake takes a write lock with a deadline.
        Intake::Operational => write_operational_record(repo, &SystemClock, &request.record)
            .map_err(|e| map_intake(&e, OP))?,
    };
    path_payload(&path, OP)
}
