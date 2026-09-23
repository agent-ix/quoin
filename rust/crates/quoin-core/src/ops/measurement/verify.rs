// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `measurement.verify`: the independent verdict checker, over a real store
//! (FR-108, PLAT-961).
//!
//! The checker itself is `quoin_measurement::verify`, which is pure. This
//! route does its I/O — load the plans, read every retained collection — and
//! hands them over with the intake order the caller supplies. The store
//! records no intake order of its own; `quoin measurement verify` supplies the
//! order in which git first added each collection file, which the producer
//! cannot choose after the fact the way it chooses a `timestamp`.
//!
//! An `accept` is a clean success. A `reject` or `inconclusive` is a complete
//! payload with a `CORE_NOT_ACCEPTED` diagnostic and exit 1, so a gate can
//! stop on the status without parsing the verdict.

use std::collections::BTreeMap;
use std::path::Path;

use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::store::read_measurement_collections;
use quoin_measurement::{Ranked, Verdict, verdict_json, verify as check};

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

use super::taxonomy::map_measurement;
use super::wire::{MAX_VERIFY_REQUEST_BYTES, VerifyRequest};
use super::{bound, parse};

/// Answer a `measurement.verify`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`VerifyRequest`] or
///   names an unknown claimed verdict.
/// - [`CoreErrorCode::Refused`] when the request exceeds its ceiling, no plan
///   has the requested id, or the store cannot be read.
pub fn verify(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.verify";
    bound(request, OP, MAX_VERIFY_REQUEST_BYTES)?;
    let request: VerifyRequest = parse(request, OP)?;
    let claimed = request
        .claimed
        .as_deref()
        .map(|claimed| {
            Verdict::from_wire(claimed).ok_or_else(|| {
                CoreError::new(
                    CoreErrorCode::BadRequest,
                    format!("unknown claimed verdict `{claimed}`; expected accept, reject or inconclusive"),
                )
                .with_context("op", OP)
            })
        })
        .transpose()?;
    let source = DiskMeasurement::new(Path::new(&request.repo));
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())
        .map_err(|error| map_measurement(&error, OP))?;
    let plan = plans
        .iter()
        .find(|plan| plan.id.as_str() == request.plan)
        .ok_or_else(|| {
            CoreError::new(
                CoreErrorCode::Refused,
                format!("no MeasurementPlan has id `{}`", request.plan),
            )
            .with_context("op", OP)
        })?;
    let collections =
        read_measurement_collections(&source).map_err(|error| map_measurement(&error, OP))?;
    // The first group a collection id appears in is its position; a later
    // repeat of the same id is ignored rather than moving it.
    let mut positions = BTreeMap::new();
    for (position, group) in (0_u64..).zip(&request.intake_order) {
        for id in group {
            positions.entry(id.as_str()).or_insert(position);
        }
    }
    let ranked: Vec<Ranked<'_>> = collections
        .iter()
        .map(|collection| Ranked {
            collection,
            intake: positions.get(collection.collection_id.as_str()).copied(),
        })
        .collect();
    let verdict = check(plan, &ranked, claimed);
    let payload = verdict_json(&verdict).map_err(|error| map_measurement(&error, OP))?;
    if verdict.verdict == Verdict::Accept {
        return Ok(Response::ok(payload));
    }
    let reasons: Vec<&str> = verdict
        .reasons
        .iter()
        .map(|reason| reason.as_str())
        .collect();
    let diagnostic = CoreError::new(
        CoreErrorCode::NotAccepted,
        format!("{} is {}", verdict.plan_id, verdict.verdict.as_str()),
    )
    .with_context("op", OP)
    .with_context("verdict", verdict.verdict.as_str())
    .with_context("reasons", reasons.join(","));
    Ok(Response::partial(payload, &diagnostic))
}
