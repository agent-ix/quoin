// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What one FR looks like as input to the lens (PLAT-837).
//!
//! This lens "reads spec text only" (PLAT-837's own words): no code, no
//! trace tags, no AST. [`FrContext`] is the whole boundary between "spec
//! text a caller read off disk" and "what this crate sends to Jev" -- it
//! carries nothing this crate cannot justify sending as `state`.

use serde::Serialize;
use typesafe_sdk_questions::Entry;

/// One acceptance-criterion row, read verbatim from the spec.
#[derive(Debug, Clone, Serialize)]
pub struct AcRow {
    /// The row's id, e.g. `FR-079-AC-4`.
    pub id: String,
    /// The row's own text, unmodified.
    pub text: String,
}

/// One FR's context: its statement, prose, and AC set.
///
/// PLAT-837's request shape: "State: the FR statement, the AC row, the FR's
/// Description/Behaviour/Constraints." All of it goes into one Jev call's
/// `state`, and one question block is asked per AC row within it -- see
/// [`crate::lens::build_request`].
#[derive(Debug, Clone, Serialize)]
pub struct FrContext {
    /// The FR's id, e.g. `FR-079`.
    pub fr_id: String,
    /// The FR's normative statement (its `shall` sentence).
    pub statement: String,
    /// The FR's Description section, when present.
    pub description: Option<String>,
    /// The FR's Behaviour section, when present.
    pub behaviour: Option<String>,
    /// The FR's Constraints section, when present.
    pub constraints: Option<String>,
    /// Every AC row under this FR, in document order.
    pub acceptance_criteria: Vec<AcRow>,
}

impl FrContext {
    /// Every AC id under this FR, in order -- what
    /// [`crate::question_set::QuestionSet::questions_for_fr`] and
    /// [`crate::verdict::extract`] both key off.
    #[must_use]
    pub fn ac_ids(&self) -> Vec<String> {
        self.acceptance_criteria
            .iter()
            .map(|row| row.id.clone())
            .collect()
    }
}

impl From<&FrContext> for Entry {
    /// Builds the `state` entry PLAT-837's request shape asks for.
    ///
    /// A plain JSON object, not a formatted string: the API accepts rich
    /// JSON anywhere it accepts a description (`typesafe-sdk-questions`'s own
    /// `Entry` doc), and a structured object keeps the FR statement, its
    /// prose sections and its AC rows addressable rather than concatenated
    /// into text Jev would have to re-parse.
    fn from(context: &FrContext) -> Self {
        serde_json::json!({
            "fr_id": context.fr_id,
            "statement": context.statement,
            "description": context.description,
            "behaviour": context.behaviour,
            "constraints": context.constraints,
            "acceptance_criteria": context.acceptance_criteria,
        })
        .into()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{AcRow, FrContext};
    use typesafe_sdk_questions::Entry;

    /// Provenance: PLAT-837
    #[test]
    fn ac_ids_preserve_document_order() {
        let context = FrContext {
            fr_id: "FR-001".to_owned(),
            statement: "The system SHALL do a thing.".to_owned(),
            description: None,
            behaviour: None,
            constraints: None,
            acceptance_criteria: vec![
                AcRow {
                    id: "FR-001-AC-1".to_owned(),
                    text: "first".to_owned(),
                },
                AcRow {
                    id: "FR-001-AC-2".to_owned(),
                    text: "second".to_owned(),
                },
            ],
        };
        assert_eq!(context.ac_ids(), vec!["FR-001-AC-1", "FR-001-AC-2"]);
    }

    /// Provenance: PLAT-837. `state` carries exactly the fields the request
    /// shape names -- FR statement, AC rows, Description/Behaviour/Constraints
    /// -- and nothing this lens was not told to send.
    #[test]
    fn state_carries_the_documented_fields_only() {
        let context = FrContext {
            fr_id: "FR-001".to_owned(),
            statement: "The system SHALL do a thing.".to_owned(),
            description: Some("desc".to_owned()),
            behaviour: None,
            constraints: None,
            acceptance_criteria: vec![AcRow {
                id: "FR-001-AC-1".to_owned(),
                text: "first".to_owned(),
            }],
        };
        let entry: Entry = (&context).into();
        let value = &entry.0;
        let object = value.as_object().expect("state is a JSON object");
        let mut keys: Vec<_> = object.keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "acceptance_criteria",
                "behaviour",
                "constraints",
                "description",
                "fr_id",
                "statement",
            ]
        );
    }
}
