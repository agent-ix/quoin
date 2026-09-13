// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The normalized shape `--results` has always accepted.
//!
//! Kept as an adapter so the default path goes through the same seam as every
//! other format. Named `entries` rather than `quoin` because that is what the
//! payload calls itself, and because it is the shape a consumer writes by hand
//! when no adapter fits their tool — the escape hatch that keeps the registry
//! from being a gate.

use serde::Deserialize;

use super::AdapterResult;
use crate::error::EvidenceError;
use crate::types::RunEntry;

/// The one message this adapter uses for a shape it cannot read, matching the
/// retained TypeScript byte for byte.
const SHAPE: &str = r#"expected JSON of the form {"entries": [{symbol, outcome, traceIds?}]}"#;

#[derive(Deserialize)]
struct Document {
    entries: Option<Vec<RunEntry>>,
}

/// Parse the normalized `{"entries": [...]}` payload.
///
/// # Divergence from the retained TypeScript, deliberately kept
///
/// `src/evidence/adapters/registry.ts` casts — `parsed.entries as RunEntry[]` —
/// so an element with no `symbol`, or an `outcome` outside the four-word union,
/// reaches the store unchecked and is written there. This port deserializes into
/// [`RunEntry`] and refuses such a document at intake, under the same message.
/// Recorded in quoin#456 rather than silently reproduced: writing an entry whose
/// outcome no reader can branch on is the intake defect FR-034 exists to stop,
/// and a port is not the place to re-introduce it quietly.
///
/// # Errors
///
/// Refuses input that is not JSON, and input whose `entries` is absent, is not
/// an array, or holds an element that is not a run entry.
pub fn parse_entries(raw: &str) -> Result<AdapterResult, EvidenceError> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|error| {
        EvidenceError::adapter("entries", format!("input is not JSON: {error}"))
    })?;
    let document: Document =
        serde_json::from_value(value).map_err(|_| EvidenceError::adapter("entries", SHAPE))?;
    let entries = document
        .entries
        .ok_or_else(|| EvidenceError::adapter("entries", SHAPE))?;
    Ok(AdapterResult::from_entries(entries))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::parse_entries;
    use crate::types::Outcome;

    #[test]
    fn reads_the_normalized_shape() {
        let result = parse_entries(
            r#"{"entries":[{"symbol":"a::b","outcome":"pass","traceIds":["FR-1-AC-1"]}]}"#,
        )
        .unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].outcome, Outcome::Pass);
        assert_eq!(
            result.entries[0].trace_ids.as_deref(),
            Some(["FR-1-AC-1".to_owned()].as_slice())
        );
        assert!(result.unrepresented.is_none());
    }

    #[test]
    fn refuses_a_document_with_no_entries_array() {
        for text in [r#"{"results":[]}"#, r#"{"entries":{}}"#, "[]"] {
            let error = parse_entries(text).unwrap_err();
            assert!(
                error
                    .to_string()
                    .starts_with("entries: expected JSON of the form"),
                "{text} => {error}"
            );
        }
    }

    #[test]
    fn refuses_input_that_is_not_json() {
        let error = parse_entries("not json").unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("entries: input is not JSON: ")
        );
    }
}
