// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `measurement`: publishing measurement evidence, producing it from
//! retained exports, and the reports read off the store (quoin#478, Stage 6).
//!
//! Exposes `quoin-measurement` and `quoin-measurement-graph` through the
//! boundary. What a collection, an intervention record and an operational
//! record are, when one may be published, what a report says and how it
//! renders is decided by those two crates and reported through here. This
//! module routes; it decides nothing a library already decides.
//!
//! # No host capability, and why this domain differs
//!
//! `ops::modules`, `ops::semantic`, `ops::change_assurance` and `ops::evidence`
//! each take a host trait from `main.rs` and name no path. This domain does
//! not, and the divergence is deliberate rather than an oversight.
//!
//! A host seam earns its keep when SOME of a domain's work is filesystem-free,
//! so the seam separates the deciding half from the reading half. Here there is
//! no such half: **all thirteen routes open a measurement store**, and every one
//! of them opens it through `quoin-measurement`'s own seams —
//! [`MeasurementSource`](quoin_measurement::MeasurementSource) and
//! [`Clock`](quoin_measurement::source::Clock) — which that crate already
//! unit-tests against `MemoryMeasurement`. A `MeasurementHost` here would
//! therefore be thirteen pass-through methods and a second copy of the routing
//! table, whose only content would be "call the function of the same name".
//! The duplication has a failure mode: a route added to the table and not to
//! the host, or added to both and spelled differently.
//!
//! What the containment audit actually forbids is this crate's library half
//! NAMING a host capability (`tests/tc_library_containment.rs`), and nothing
//! under `ops/measurement/` names `std::fs`, `std::env`, `std::process` or
//! `std::net`. A repository root arrives as a string and is handed to
//! `quoin-measurement` as a [`Path`]; every path in a payload is one that crate
//! produced. The disk-touching coverage of these routes is therefore the
//! integration suite `tests/tc_478_measurement_dispatch.rs`, which drives the
//! real binary against a real store, rather than a unit test against a double.
//!
//! # Why `build_*` and `render_*` both read the store
//!
//! `assurance.build_case` hands its caller a case and `assurance.render_case`
//! takes one back, because `quoin_assurance::RenderableCase` is
//! `Deserialize`. The measurement report types are not: `MeasurementReport`,
//! `MeasurementComparisonReport`, `PortfolioReport` and
//! `GovernedGraphPortfolioReport` derive `Clone, Debug, PartialEq` and nothing
//! else, and their canonical JSON is produced by private wire structs inside
//! each crate's `render_json` module.
//!
//! So a `render_*` route here takes the same LOCATING request its `build_*`
//! partner does and reads the store again, rather than taking a report back
//! over the wire. One invocation is one operation, so no in-memory report
//! survives between two calls in any case; and making the report types
//! `Deserialize` purely to pass them back would put a second declaration of
//! every report shape on this boundary, which is the thing
//! `ops::evidence::wire`'s own rule forbids.
//!
//! The domain is split the way it reads: [`wire`] holds the request and payload
//! shapes together with the ceilings they are read under, [`taxonomy`] holds the
//! one place a library failure becomes an exit status, and the four operation
//! modules hold one group of routes each.

mod portfolio;
mod produce;
mod record;
mod report;
mod taxonomy;
mod verify;
mod wire;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

pub use self::wire::{
    AgentEvalRequest, ComparisonRequest, GitHubReleaseRequest, GraphPortfolioRequest,
    MAX_ASSURANCE_DOCUMENT_BYTES, MAX_COLLECTION_BYTES, MAX_GRAPH_MAPPING_BYTES,
    MAX_INTERVENTION_RECORD_BYTES, MAX_METRIC_NAME_BYTES, MAX_OPERATIONAL_RECORD_BYTES,
    MAX_PORTFOLIO_ROOTS_BYTES, MAX_PRODUCER_DEFINITION_BYTES, MAX_RECORD_ID_BYTES,
    MAX_RETAINED_EXPORT_BYTES, MAX_REVISION_BYTES, MAX_VERIFY_REQUEST_BYTES,
    MAX_WORKFLOW_YAML_BYTES, PathPayload, PortfolioRequest, RecordRequest, RenderedPayload,
    RepoRequest, SeriesRequest, VerifyRequest,
};

pub use self::portfolio::{
    build_graph_portfolio, build_portfolio, render_graph_portfolio, render_portfolio,
};
// `ops::change_assurance` reuses this mapping to resolve a
// `change_assurance.receipt` plan link through `quoin-measurement`'s own
// plan intake (PLAT-997), rather than restating `MeasurementErrorCode`'s
// BadRequest/Refused split a second time. Re-exported rather than making
// `taxonomy` itself `pub(crate)`, which would desync
// `dispatch::tests::the_bound_census_can_see_every_ops_module`'s source-text
// module scan (it recognises only bare `mod x;`/`pub mod x;`).
pub use self::produce::{produce_agent_eval_intervention, produce_github_release_operational};
pub use self::record::record;
pub use self::report::{
    build_comparison, build_report, build_series, render_comparison, render_report,
};
pub(crate) use self::taxonomy::map_measurement;
pub use self::verify::verify;

/// Refuse an oversized request before it is deserialised.
fn bound(request: &serde_json::Value, op: &'static str, limit: usize) -> Result<(), CoreError> {
    let size = request_size(request)?;
    if size > limit {
        return Err(refusal(op, limit, size));
    }
    Ok(())
}

/// Refuse an oversized request FIELD before any work is done on it.
///
/// Separate from [`bound`] because the two refusals answer different questions:
/// a whole-request refusal says "you sent too much", a field refusal says
/// "this member is too big", and a caller can only act on the second.
fn bound_field(
    op: &'static str,
    field: &'static str,
    size: usize,
    limit: usize,
) -> Result<(), CoreError> {
    if size > limit {
        return Err(CoreError::new(
            CoreErrorCode::Refused,
            "a request field exceeds the accepted size",
        )
        .with_context("op", op)
        .with_context("field", field)
        .with_context("limit_bytes", limit.to_string())
        .with_context("observed_bytes", size.to_string()));
    }
    Ok(())
}

/// Parse a request, naming the operation on the refusal.
fn parse<T: for<'de> Deserialize<'de>>(
    request: &serde_json::Value,
    op: &'static str,
) -> Result<T, CoreError> {
    serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
    })
}

/// Serialise a payload into a clean success.
fn ok<T: Serialize>(payload: &T) -> Result<Response, CoreError> {
    let value = serde_json::to_value(payload)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;
    Ok(Response::ok(value))
}

/// The canonical JSON a `render_json` function produced, as a payload.
///
/// Every `build_*` route answers with the document its crate's own
/// `render_json` defines, read back through `serde_json`. The alternative —
/// serialising the report's wire struct here — would make this boundary a
/// SECOND definition of each document, and the first to disagree with the file
/// the retained TypeScript writes. The round trip costs one parse and buys the
/// property that `build_report` and `renderMeasurementReportJson` cannot drift.
fn document(canonical: &str, op: &'static str) -> Result<Response, CoreError> {
    let value: serde_json::Value = serde_json::from_str(canonical).map_err(|e| {
        CoreError::new(
            CoreErrorCode::Io,
            format!("the rendered document is not JSON: {e}"),
        )
        .with_context("op", op)
    })?;
    Ok(Response::ok(value))
}

/// A produced or published path, as a payload.
fn path_payload(path: &Path, op: &'static str) -> Result<Response, CoreError> {
    ok(&wire::PathPayload {
        path: path.to_string_lossy().into_owned(),
    })
    .map_err(|error| error.with_context("op", op))
}

/// A rendered document, as a payload.
fn rendered(rendered: String, op: &'static str) -> Result<Response, CoreError> {
    ok(&wire::RenderedPayload { rendered }).map_err(|error| error.with_context("op", op))
}

/// The length of one string member, or zero when it is absent or not a string.
///
/// Zero rather than a refusal: a member that is absent or of the wrong type is
/// [`parse`]'s verdict to give, and it gives a better one. This answers only
/// "how big", and nothing is too big at zero.
fn string_len(request: &serde_json::Value, field: &str) -> usize {
    request
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map_or(0, str::len)
}

/// The repository roots a portfolio request names.
fn roots(locations: &[String]) -> Vec<PathBuf> {
    locations.iter().map(PathBuf::from).collect()
}
