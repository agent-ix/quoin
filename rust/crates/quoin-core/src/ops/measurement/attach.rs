// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `measurement.attach`: retain one repository file as an immutable
//! content-addressed attachment (FR-125).
//!
//! The path judgement (safe relative path, no symlinked component, regular
//! file, size bound) is `quoin_measurement::retain_attachment_file`'s, and runs
//! before any byte is loaded. This route names no host capability.

use std::path::Path;

use quoin_measurement::retain_attachment_file;

use crate::error::CoreError;
use crate::protocol::Response;

use super::taxonomy::map_measurement;
use super::wire::{AttachRequest, MAX_VERIFY_REQUEST_BYTES};
use super::{bound, parse};

/// Answer a `measurement.attach` with the attachment reference to state in a
/// collection's `attachments` member.
///
/// # Errors
///
/// - [`crate::error::CoreErrorCode::BadRequest`] when stdin is not an
///   [`AttachRequest`] or the path or file is unacceptable.
/// - [`crate::error::CoreErrorCode::Refused`] when the store declines.
pub fn attach(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.attach";
    bound(request, OP, MAX_VERIFY_REQUEST_BYTES)?;
    let request: AttachRequest = parse(request, OP)?;
    let reference =
        retain_attachment_file(Path::new(&request.repo), &request.path, &request.media_type)
            .map_err(|error| map_measurement(&error, OP))?;
    let json = reference
        .to_json()
        .map_err(|error| map_measurement(&error, OP))?;
    let value = quoin_measurement::json_bridge::to_serde(&json)
        .map_err(|error| map_measurement(&error, OP))?;
    Ok(Response::ok(value))
}
