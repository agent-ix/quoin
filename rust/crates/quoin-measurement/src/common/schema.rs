// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The schema pass both record families run, written once.
//!
//! `intervention.ts:51-62` and `operational.ts:57-66` are the same eight lines
//! twice: run the compiled ajv validator, render each error as
//! `` `${instancePath || "/"}: ${message}` ``, merge the semantic findings in,
//! then `[...new Set(findings)].sort()`. quoin#471 and quoin#472 ported them
//! independently and landed the same two helpers in
//! `intervention/validate.rs` and `operational/validate.rs`.
//!
//! They are here because the rendering is **contractual across both
//! families**: a caller reads `/<pointer>: <sentence>` and splits on the first
//! colon. Two copies are two chances for one family to start spelling the
//! empty pointer as `""` while the other spells it `"/"`, and nothing would
//! fail when that happened. The compiled validator stays in each family's own
//! module, because the schema it compiles is the one thing about the pass that
//! genuinely differs.

use quoin_jsonschema::MeasurementValidator;
use serde_json::Value;

/// The root pointer's spelling when an error carries no instance path.
///
/// `error.instancePath || "/"`: ajv writes `""` for the document itself and
/// the retained code substitutes `/`, so a caller never sees an empty pointer.
const ROOT: &str = "/";

/// Every schema failure against `candidate`, in the oracle's finding spelling.
///
/// Emission order, not sorted: [`sorted_unique`] is what orders a refusal, and
/// it runs after the semantic findings have been merged in.
pub(crate) fn findings(validator: &MeasurementValidator, candidate: &Value) -> Vec<String> {
    validator
        .errors(candidate)
        .into_iter()
        .map(|error| {
            let pointer = if error.instance_path.is_empty() {
                ROOT
            } else {
                error.instance_path.as_str()
            };
            format!("{pointer}: {}", error.message)
        })
        .collect()
}

/// `[...new Set(findings)].sort(...)` — the set, then the order.
///
/// Sorting before deduplicating reaches the same vector as deduplicating
/// before sorting, and is the cheaper of the two; what a caller is promised is
/// that two runs over one candidate compare equal.
pub(crate) fn sorted_unique(mut findings: Vec<String>) -> Vec<String> {
    findings.sort_unstable();
    findings.dedup();
    findings
}
