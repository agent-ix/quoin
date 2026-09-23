// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A change whose diff touches a plan's protected apparatus (PLAT-964,
//! quoin FR-111).
//!
//! `quoin-measurement`'s FR-110 resolves a `MeasurementPlan`'s
//! `protected_apparatus` at intake and compares the *recorded* (path, digest)
//! set across stored collections — it never looks at a diff, because a
//! collection has none. Change-assurance verifies a proposed change instead,
//! and a change has exactly the thing a collection does not: a diff. This
//! module is the other half of FR-110's promise, applied to that diff: when
//! [`VerificationInput::diff_paths`](crate::verify::input::VerificationInput::diff_paths)
//! names a path the linked plan protects, the change is refused credit
//! toward that plan's objective, whether or not the plan declared the
//! `apparatus-edit` negative control (mirroring FR-110-AC-7's "why a changed
//! set rejects": the protection is unconditional).
//!
//! The entry grammar — a file path, or a `<directory>/**` entry naming every
//! file under a directory — is engineering-assurance's own
//! (`ApparatusPath::directory`); this module states no second copy of it.
//!
//! # Negative controls this crate cannot evaluate
//!
//! A plan may declare a `negative_controls` kind this crate has no rule for
//! (`suppressed-observation`, `gain-within-noise`, `stale-evidence`,
//! `selective-reporting`): change-assurance verifies retained evidence for
//! one candidate revision, and those four are about a population, a margin,
//! an evidence's age, or a choice among several runs — none of which this
//! verification is given. Declaring one of them is not itself wrong, but
//! silently ignoring it would let a receipt claim more than it checked, so
//! each uncaught kind is its own [`Reason::NegativeControlUncaught`] and
//! leaves the outcome `incomplete` rather than `valid`.

use engineering_assurance::measurement::{
    NegativeControlKind, NegativeControls, ProtectedApparatus,
};

use crate::model::outcome::Reason;

/// Everything one apparatus judgment is drawn from.
pub struct ApparatusContext<'a> {
    /// The plan's declared protected apparatus, when it links one.
    pub protected_apparatus: Option<&'a ProtectedApparatus>,
    /// The plan's declared negative controls, when it links one.
    pub negative_controls: Option<&'a NegativeControls>,
    /// The repository-relative paths the candidate change's diff touches.
    pub diff_paths: &'a [String],
}

/// Judge the diff against the linked plan's protected apparatus and negative
/// controls.
///
/// Folded into the receipt's overall reasons the same way
/// [`crate::verify::proofs`]'s global reasons are: there is no dedicated
/// `checks` member for this, because the judgment is not about the record
/// itself but about the change proposed against a plan the record cites.
#[must_use]
pub fn judge(context: &ApparatusContext<'_>) -> Vec<Reason> {
    let mut reasons = Vec::new();
    if let Some(protected) = context.protected_apparatus
        && context
            .diff_paths
            .iter()
            .any(|path| touches(protected, path))
    {
        reasons.push(Reason::ApparatusTouched);
    }
    if let Some(controls) = context.negative_controls {
        for control in controls.as_slice() {
            if !evaluated(control.kind()) {
                reasons.push(Reason::NegativeControlUncaught);
            }
        }
    }
    reasons
}

/// Whether `path` is one of `protected`'s entries: an exact file match, or
/// under a `<directory>/**` entry's directory.
fn touches(protected: &ProtectedApparatus, path: &str) -> bool {
    protected.iter().any(|entry| match entry.directory() {
        Some(directory) => {
            path == directory
                || path
                    .strip_prefix(directory)
                    .is_some_and(|rest| rest.starts_with('/'))
        }
        None => path == entry.as_str(),
    })
}

/// Whether this crate has a rule for `kind`. Only `apparatus-edit` is
/// evaluated here — see the module doc for the other four.
const fn evaluated(kind: NegativeControlKind) -> bool {
    matches!(kind, NegativeControlKind::ApparatusEdit)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use engineering_assurance::measurement::{ApparatusPath, NegativeControl};

    use super::{
        ApparatusContext, NegativeControlKind, NegativeControls, ProtectedApparatus, Reason, judge,
    };

    fn apparatus(entries: &[&str]) -> ProtectedApparatus {
        ProtectedApparatus::new(
            entries
                .iter()
                .map(|entry| ApparatusPath::new(*entry).unwrap()),
        )
        .unwrap()
    }

    #[test]
    fn a_diff_touching_a_directory_entry_is_apparatus_touched() {
        let protected = apparatus(&["checker/config.toml", "tests/fixtures/harness/**"]);
        let diff = vec!["tests/fixtures/harness/run.rs".to_owned()];
        let reasons = judge(&ApparatusContext {
            protected_apparatus: Some(&protected),
            negative_controls: None,
            diff_paths: &diff,
        });
        assert_eq!(reasons, vec![Reason::ApparatusTouched]);
    }

    #[test]
    fn a_diff_outside_every_entry_is_clean() {
        let protected = apparatus(&["checker/config.toml", "tests/fixtures/harness/**"]);
        let diff = vec!["src/lib.rs".to_owned()];
        let reasons = judge(&ApparatusContext {
            protected_apparatus: Some(&protected),
            negative_controls: None,
            diff_paths: &diff,
        });
        assert!(reasons.is_empty());
    }

    #[test]
    fn a_declared_control_with_no_rule_is_uncaught() {
        let controls = NegativeControls::new([
            NegativeControl::new(NegativeControlKind::ApparatusEdit, "answer key").unwrap(),
            NegativeControl::new(NegativeControlKind::GainWithinNoise, "margin claims").unwrap(),
        ])
        .unwrap();
        let reasons = judge(&ApparatusContext {
            protected_apparatus: None,
            negative_controls: Some(&controls),
            diff_paths: &[],
        });
        assert_eq!(reasons, vec![Reason::NegativeControlUncaught]);
    }
}
