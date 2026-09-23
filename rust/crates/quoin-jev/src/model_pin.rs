// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one comparison every model-pin check in this crate makes (PLAT-978).
//!
//! Two paths refuse an answer from a model other than the one their caller
//! pinned: [`crate::cassette`] checks every recorded line (and every fresh
//! answer record mode appends), and [`crate::lens::run`] checks the response
//! it is about to score. Both route through [`check`] so the rule -- exact
//! string equality against the concrete version the response reports, never
//! an alias and never a prefix -- is written once. Relaxing it (say, to accept
//! any `jev-1.13.*`) is a change here, not a change to remember in two
//! modules.
//!
//! This crate does not own the pinned value. The caller does, for the same
//! reason `verdict::extract` takes `confidence_threshold` as an argument:
//! thresholds are calibrated per model version, and the caller holding the
//! calibration is the one who knows which version it was calibrated against.

use crate::error::{JevError, JevErrorCode, Result};

/// Checks that `observed`, the model a response reports, is exactly
/// `expected`, the model the caller pinned.
///
/// `code` names which path refused -- [`JevErrorCode::CassetteModelMismatch`]
/// for a cassette line, [`JevErrorCode::ModelMismatch`] for a response
/// [`crate::lens::run`] was about to score -- and `at` locates the response
/// in the message (a `path:line`, a request key, an FR id).
///
/// # Errors
/// An error carrying `code` if `observed != expected`.
pub(crate) fn check(observed: &str, expected: &str, code: JevErrorCode, at: &str) -> Result<()> {
    if observed == expected {
        return Ok(());
    }
    Err(JevError::new(
        code,
        format!("{at}: answered by model {observed:?}, but the caller pinned {expected:?}"),
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::check;
    use crate::error::JevErrorCode;

    /// Provenance: PLAT-978. The pinned model passes.
    #[test]
    fn the_pinned_model_passes() {
        assert!(
            check(
                "jev-1.13.0",
                "jev-1.13.0",
                JevErrorCode::ModelMismatch,
                "FR-1"
            )
            .is_ok()
        );
    }

    /// Provenance: PLAT-978. Equality is exact: a neighbouring patch
    /// version, a prefix, and an alias are all refused under the code the
    /// caller named, and the message names both models.
    #[test]
    fn anything_but_the_exact_pinned_model_is_refused() {
        for observed in ["jev-1.13.1", "jev-1.13", "jev-latest", ""] {
            let error = check(observed, "jev-1.13.0", JevErrorCode::ModelMismatch, "FR-1")
                .expect_err(observed);
            assert_eq!(error.code, JevErrorCode::ModelMismatch, "{observed}");
            assert!(error.message.contains("jev-1.13.0"), "{}", error.message);
            assert!(
                error.message.contains(&format!("{observed:?}")),
                "{}",
                error.message
            );
        }
    }

    /// Provenance: PLAT-978. The code is the caller's, so the cassette's
    /// refusal keeps its own code.
    #[test]
    fn the_refusal_carries_the_callers_code() {
        let error = check(
            "jev-2.0.0",
            "jev-1.13.0",
            JevErrorCode::CassetteModelMismatch,
            "fixture.jsonl:1",
        )
        .expect_err("skewed");
        assert_eq!(error.code, JevErrorCode::CassetteModelMismatch);
        assert!(
            error.message.starts_with("fixture.jsonl:1: "),
            "{}",
            error.message
        );
    }
}
