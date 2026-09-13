// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The authored assurance-argument view (FR-047, ported from
//! `buildAuthoredArgumentView` and `renderAuthoredArgument` in
//! `src/assurance/argument.ts`).
//!
//! # The one rule the whole module exists to keep
//!
//! **The renderer never promotes an evidence result into a claim.** A criterion
//! is supported only when a named participant, holding the authority the
//! argument itself declares for them, decided it — and the decision has to be
//! current at the stated `asOf`. Everything else reads `open`, with the reason
//! kept next to it. There is no score anywhere in the output and FR-047-AC-1
//! asserts its absence, because a number is precisely the thing a reader would
//! substitute for the decision.
//!
//! # What this module does NOT re-implement
//!
//! [`crate::argument`] already owns the instant grammar, the JavaScript-trim
//! set, the object/string/array readers and [`crate::ArgumentError`]. Every one of
//! them is reached through that module rather than transcribed again: a second
//! instant parser is the exact shape of quoin#436, where two readers of the
//! same timestamp disagreed and a rolled date silently decided a reported
//! status.
//!
//! The comparison the view performs needs a NUMBER, not a yes/no, so
//! `argument::instant_epoch_millis` is the shared reader and
//! `argument::is_instant` is a thin caller of it.

mod build;
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod fixtures;
mod parse;
mod render;
mod view;

pub use build::build_authored_argument_view;
pub use render::render_authored_argument;
pub use view::{
    ArgumentSummary, AssumptionView, AuthoredArgumentView, BuildAuthoredArgumentRequest,
    ChallengeView, ChallengeViewStatus, CriterionView, DecisionState, ReasoningView,
    SufficiencyDecision, TopClaimView, UnusedDecision, ViewSchemaVersion, ViewStatus,
};
