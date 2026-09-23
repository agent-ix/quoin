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
//! payload with a `CORE_REJECTED` or `CORE_INCONCLUSIVE` diagnostic and exit
//! 1, so a gate can stop on the status without parsing the verdict.
//!
//! A collection whose file name is not its `collectionId` is refused: the
//! intake order is keyed by file name and the checker by id, and a file that
//! names one collection while holding another would take a position that is
//! not its own.

use std::collections::BTreeMap;
use std::path::Path;

use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::store::read_measurement_collection_results;
use quoin_measurement::{OrderSource, Ranked, TamperFacts, Verdict, verdict_json, verify as check};

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

use super::taxonomy::map_measurement;
use super::wire::{MAX_VERIFY_REQUEST_BYTES, TamperedCollectionRequest, VerifyRequest};
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
    let order = match request.order_source.as_deref() {
        Some(stated) => OrderSource::from_wire(stated).ok_or_else(|| {
            CoreError::new(
                CoreErrorCode::BadRequest,
                format!("unknown order source `{stated}`"),
            )
            .with_context("op", OP)
        })?,
        None if request.intake_order.is_empty() => OrderSource::None,
        None => OrderSource::CallerSupplied,
    };
    let collections = collections(&source)?;
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
            apparatus_forged: request
                .apparatus_forged
                .iter()
                .any(|id| id == collection.collection_id.as_str()),
        })
        .collect();
    // A deleted or edited collection is attributed to this plan when the
    // caller read this plan's id among the observations it carried, in any
    // content it had (PLAT-985); the caller alone has git and did that
    // reading.
    let naming_plan = |tampered: &[TamperedCollectionRequest]| -> Vec<String> {
        tampered
            .iter()
            .filter(|entry| entry.plan_ids.iter().any(|id| id == plan.id.as_str()))
            .map(|entry| entry.id.clone())
            .collect()
    };
    let deleted_collections = naming_plan(&request.deleted);
    let edited_collections = naming_plan(&request.edited_collections);
    let tamper = TamperFacts {
        deleted_collections: &deleted_collections,
        edited_collections: &edited_collections,
        definition_changed_without_version_bump: request.definition_changed_without_version_bump,
    };
    let verdict = check(plan, &ranked, tamper, order, claimed);
    let payload = verdict_json(&verdict).map_err(|error| map_measurement(&error, OP))?;
    if verdict.verdict == Verdict::Accept {
        return Ok(Response::ok(payload));
    }
    let reasons: Vec<&str> = verdict
        .reasons
        .iter()
        .map(|reason| reason.as_str())
        .collect();
    let code = if verdict.verdict == Verdict::Reject {
        CoreErrorCode::Rejected
    } else {
        CoreErrorCode::Inconclusive
    };
    let diagnostic = CoreError::new(
        code,
        format!("{} is {}", verdict.plan_id, verdict.verdict.as_str()),
    )
    .with_context("op", OP)
    .with_context("verdict", verdict.verdict.as_str())
    .with_context("reasons", reasons.join(","));
    Ok(Response::partial(payload, &diagnostic))
}

/// Every retained collection, refusing the read when one is unreadable or
/// its file name is not its `collectionId`.
fn collections(
    source: &DiskMeasurement,
) -> Result<Vec<quoin_measurement::MeasurementCollection>, CoreError> {
    const OP: &str = "measurement.verify";
    let mut out = Vec::new();
    for result in
        read_measurement_collection_results(source).map_err(|error| map_measurement(&error, OP))?
    {
        let collection = result.collection.map_err(|error| {
            CoreError::new(
                CoreErrorCode::Refused,
                format!(
                    "{}: unreadable measurement collection: {error}",
                    result.path
                ),
            )
            .with_context("op", OP)
        })?;
        let named = Path::new(&result.path)
            .file_stem()
            .and_then(std::ffi::OsStr::to_str);
        if named != Some(collection.collection_id.as_str()) {
            return Err(CoreError::new(
                CoreErrorCode::Refused,
                format!(
                    "{} holds collection `{}`; a collection's file name must be its collectionId",
                    result.path, collection.collection_id
                ),
            )
            .with_context("op", OP));
        }
        out.push(collection);
    }
    Ok(out)
}
