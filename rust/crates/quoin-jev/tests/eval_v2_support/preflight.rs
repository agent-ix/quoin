// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What the runner checks before it spends a single call (PR #620 review and
//! re-review, findings 3a, 4 and 5). Both the live runner and the offline
//! tests call [`authorize_run`], so deleting a check from it fails an
//! offline test, not only a live run nobody can afford to repeat.
//!
//! - No variant may be past the dev cap of [`MAX_DEV_VERSIONS`] versions.
//! - On the held-out split: the flag and the seal
//!   ([`corpus::authorize_heldout`]), a committed selection entry covering
//!   every variant ([`corpus::check_heldout_selected`]), and no unexplained
//!   rerun ([`corpus::check_heldout_rerun`]).

use std::path::Path;

use super::corpus::{self, Source, Split};
use super::variant::Variant;

/// The most versions a variant family may have on dev (PLAT-1024 rule 5,
/// MP-240). A version past it is refused, not merely reported.
pub(crate) const MAX_DEV_VERSIONS: u32 = 5;

/// Refuses any variant past [`MAX_DEV_VERSIONS`].
///
/// # Errors
/// Naming every such variant.
pub(crate) fn check_version_cap(variants: &[&Variant]) -> Result<(), String> {
    let over: Vec<String> = variants
        .iter()
        .filter(|variant| variant.version > MAX_DEV_VERSIONS)
        .map(|variant| variant.label())
        .collect();
    if over.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{over:?} are past the dev cap of {MAX_DEV_VERSIONS} versions per family; the \
             family's last capped version is its final one"
        ))
    }
}

/// The run's settings the preflight reads.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunGate<'a> {
    /// The split to run.
    pub(crate) split: Split,
    /// The value of [`corpus::HELDOUT_ENV`].
    pub(crate) heldout_flag: Option<&'a str>,
    /// The value of [`corpus::HELDOUT_RERUN_ENV`].
    pub(crate) rerun_reason: Option<&'a str>,
    /// The held-out selection file ([`corpus::heldout_selection_path`] in a
    /// real run).
    pub(crate) selection: &'a Path,
    /// The held-out run log ([`corpus::heldout_log_path`] in a real run).
    pub(crate) log: &'a Path,
}

/// Every check a run must pass before its first request. Returns each
/// sealed source's held-out digest (empty on dev).
///
/// # Errors
/// The first check that fails, naming what it refused.
pub(crate) fn authorize_run(
    gate: &RunGate<'_>,
    variants: &[&Variant],
    sources: &[&Source],
) -> Result<Vec<String>, String> {
    check_version_cap(variants)?;
    if gate.split != Split::Heldout {
        return Ok(Vec::new());
    }
    let seals = corpus::authorize_heldout(gate.heldout_flag, sources)?;
    let text = std::fs::read_to_string(gate.selection)
        .map_err(|error| format!("{}: {error}", gate.selection.display()))?;
    let selection = corpus::parse_selection(&text)?;
    let chosen: Vec<(&str, u32)> = variants
        .iter()
        .map(|variant| (variant.id, variant.version))
        .collect();
    corpus::check_heldout_selected(&selection, &chosen)?;
    let labels: Vec<String> = variants.iter().map(|variant| variant.label()).collect();
    corpus::check_heldout_rerun(gate.log, &labels, gate.rerun_reason)?;
    Ok(seals)
}
