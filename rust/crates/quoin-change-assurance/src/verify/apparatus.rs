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
//! A plan that protects apparatus, checked against a verification that
//! retained no diff, is [`Reason::DiffMissing`] rather than clean: an absent
//! diff was never compared against anything, and passing it would make the
//! protection vacuous for any caller that simply omits the diff.
//!
//! The entry grammar — a file path, or a `<directory>/**` entry naming every
//! file under a directory — is engineering-assurance's own
//! (`ApparatusPath::directory`); this module states no second copy of it.
//! Matching is exact and case-sensitive, as FR-110's intake resolution is.
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
//! an uncaught kind is [`Reason::NegativeControlUncaught`] and leaves the
//! outcome `incomplete` rather than `valid`. An `apparatus-edit` control with
//! no `protected_apparatus` list has nothing to guard (`quoin-measurement`'s
//! plan intake refuses that combination outright) and is uncaught too.

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
    /// The repository-relative paths the candidate change's diff touches;
    /// `None` when no diff was retained.
    pub diff_paths: Option<&'a [String]>,
}

/// Judge the diff against the linked plan's protected apparatus and negative
/// controls.
///
/// Folded into the receipt's overall reasons the same way
/// [`crate::verify::proofs`]'s global reasons are: there is no dedicated
/// `checks` member for this, because the judgment is not about the record
/// itself but about the change proposed against a plan the record cites.
/// Each reason appears at most once; the receipt's reasons are a set.
#[must_use]
pub fn judge(context: &ApparatusContext<'_>) -> Vec<Reason> {
    let mut reasons = Vec::new();
    if let Some(protected) = context.protected_apparatus {
        match context.diff_paths {
            None => reasons.push(Reason::DiffMissing),
            Some(paths) if paths.iter().any(|path| touches(protected, path)) => {
                reasons.push(Reason::ApparatusTouched);
            }
            Some(_) => {}
        }
    }
    let guarded = context.protected_apparatus.is_some();
    if context.negative_controls.is_some_and(|controls| {
        controls
            .as_slice()
            .iter()
            .any(|control| !evaluated(control.kind(), guarded))
    }) {
        reasons.push(Reason::NegativeControlUncaught);
    }
    reasons
}

/// Whether `path` is one of `protected`'s entries: an exact file match, or
/// under a `<directory>/**` entry's directory. A path equal to a directory
/// entry's directory also matches — a file replacing a protected directory
/// is still an edit to it.
fn touches(protected: &ProtectedApparatus, path: &str) -> bool {
    protected.iter().any(|entry| match entry.directory() {
        Some(directory) => path
            .strip_prefix(directory)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/')),
        None => path == entry.as_str(),
    })
}

/// Whether this crate has a rule for `kind`. Only `apparatus-edit` is
/// evaluated here, and only when there is a protected list for it to guard —
/// see the module doc for the other four.
const fn evaluated(kind: NegativeControlKind, guarded: bool) -> bool {
    matches!(kind, NegativeControlKind::ApparatusEdit) && guarded
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

    fn judged(protected: &ProtectedApparatus, diff: &[&str]) -> Vec<Reason> {
        let diff: Vec<String> = diff.iter().map(|path| (*path).to_owned()).collect();
        judge(&ApparatusContext {
            protected_apparatus: Some(protected),
            negative_controls: None,
            diff_paths: Some(&diff),
        })
    }

    #[test]
    fn a_diff_touching_a_directory_entry_is_apparatus_touched() {
        let protected = apparatus(&["checker/config.toml", "tests/fixtures/harness/**"]);
        assert_eq!(
            judged(&protected, &["tests/fixtures/harness/deep/run.rs"]),
            vec![Reason::ApparatusTouched]
        );
        assert_eq!(
            judged(&protected, &["tests/fixtures/harness"]),
            vec![Reason::ApparatusTouched]
        );
    }

    #[test]
    fn a_diff_outside_every_entry_is_clean() {
        let protected = apparatus(&["checker/config.toml", "tests/fixtures/harness/**"]);
        assert!(judged(&protected, &["src/lib.rs"]).is_empty());
        // A sibling sharing the directory's prefix is not under it.
        assert!(judged(&protected, &["tests/fixtures/harness-old/run.rs"]).is_empty());
        // A file entry names that file only, not a directory of that name.
        assert!(judged(&protected, &["checker/config.toml.bak"]).is_empty());
        // Matching is case-sensitive, as FR-110's resolution is.
        assert!(judged(&protected, &["Checker/config.toml"]).is_empty());
        // An empty diff touched nothing; it was still retained and compared.
        assert!(judged(&protected, &[]).is_empty());
    }

    #[test]
    fn a_protecting_plan_with_no_retained_diff_is_diff_missing() {
        let protected = apparatus(&["checker/config.toml"]);
        let reasons = judge(&ApparatusContext {
            protected_apparatus: Some(&protected),
            negative_controls: None,
            diff_paths: None,
        });
        assert_eq!(reasons, vec![Reason::DiffMissing]);
    }

    #[test]
    fn a_declared_control_with_no_rule_is_uncaught_once() {
        let controls = NegativeControls::new([
            NegativeControl::new(NegativeControlKind::ApparatusEdit, "answer key").unwrap(),
            NegativeControl::new(NegativeControlKind::GainWithinNoise, "margin claims").unwrap(),
            NegativeControl::new(NegativeControlKind::StaleEvidence, "old runs").unwrap(),
        ])
        .unwrap();
        let protected = apparatus(&["checker/config.toml"]);
        let reasons = judge(&ApparatusContext {
            protected_apparatus: Some(&protected),
            negative_controls: Some(&controls),
            diff_paths: Some(&[]),
        });
        assert_eq!(reasons, vec![Reason::NegativeControlUncaught]);
    }

    #[test]
    fn an_apparatus_edit_control_with_nothing_to_guard_is_uncaught() {
        let controls = NegativeControls::new([NegativeControl::new(
            NegativeControlKind::ApparatusEdit,
            "answer key",
        )
        .unwrap()])
        .unwrap();
        let reasons = judge(&ApparatusContext {
            protected_apparatus: None,
            negative_controls: Some(&controls),
            diff_paths: Some(&[]),
        });
        assert_eq!(reasons, vec![Reason::NegativeControlUncaught]);
    }
}
