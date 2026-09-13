// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Admit a `quire-assurance` export under premises the caller already agreed to.
//!
//! Ports `adaptQuireAssurance` (`src/measurement/graph-adapters.ts:423-448`).
//!
//! # Two checks, in the retained order
//!
//! 1. The **published contract**: [`quoin_quire::validate_assurance`], which is
//!    the Rust successor to `validateAssurance` and the reason quoin#474
//!    vendored `assurance-v1` at all.
//! 2. The **premises**: the export's own `source` and `modules` must be the
//!    ones the caller stated. An export that satisfies the schema but was taken
//!    from a different revision is a valid document and the wrong evidence.
//!
//! The order is kept because the first refusal is the one a caller sees, and
//! a document that is not an `assurance-v1` document at all should be refused
//! as that rather than as a premise disagreement.
//!
//! # The two checks the port cannot fail
//!
//! `graph-adapters.ts:432-440` also compares `format` and `format_version`
//! against the accepted premises. Both sides of both comparisons are TypeScript
//! *literal* types — `"quire-assurance"` and `1` — so the only way to reach a
//! failure there is a cast that lies. Here they are
//! [`crate::wire::QuireAssuranceFormat`] and [`crate::wire::FormatVersion1`],
//! types with one inhabitant each, so the disagreement is unrepresentable and
//! the branch is absent rather than dead.

use serde::Deserialize as _;
use serde_json::Value;

use crate::canonical::pretty_text_trimmed;
use crate::error::{GraphAdapterError, GraphAdapterErrorCode, Result};

use super::QuireAssuranceV1;
use super::premise::AcceptedQuirePremises;

fn refuse(detail: impl Into<String>) -> GraphAdapterError {
    GraphAdapterError::new(GraphAdapterErrorCode::InvalidPremise, detail)
}

/// Validate an export against the published schema and the caller's premises.
///
/// # Errors
///
/// [`GraphAdapterErrorCode::InvalidPremise`] when the document does not satisfy
/// `assurance-v1`, when it does not satisfy the adapter's stricter shape, and
/// when its `source` or `modules` disagree with `accepted`.
pub fn adapt_quire_assurance(
    document: &Value,
    accepted: &AcceptedQuirePremises,
) -> Result<QuireAssuranceV1> {
    quoin_quire::validate_assurance(document.clone()).map_err(|error| refuse(error.to_string()))?;
    let parsed =
        QuireAssuranceV1::deserialize(document).map_err(|error| refuse(error.to_string()))?;

    if parsed.source != accepted.source {
        return Err(premise_failure("source", &accepted.source, &parsed.source)?);
    }
    if parsed.modules != accepted.modules {
        return Err(premise_failure(
            "modules",
            &accepted.modules,
            &parsed.modules,
        )?);
    }
    Ok(parsed)
}

/// Render the refusal `premiseFailure` renders: the member, then both sides in
/// canonical JSON.
///
/// Returns the error rather than raising it so the caller's `return Err(…)`
/// stays visible at the branch it belongs to.
fn premise_failure<T: serde::Serialize>(
    member: &str,
    expected: &T,
    observed: &T,
) -> Result<GraphAdapterError> {
    let render = |value: &T| -> Result<String> {
        let as_value = serde_json::to_value(value).map_err(|error| {
            GraphAdapterError::new(GraphAdapterErrorCode::Store, error.to_string())
        })?;
        pretty_text_trimmed(&as_value)
    };
    Ok(refuse(format!(
        "{member} expected {}; observed {}",
        render(expected)?,
        render(observed)?
    )))
}
