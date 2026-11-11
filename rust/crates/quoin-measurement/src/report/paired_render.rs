// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The "Paired runs" table (quoin FR-132-AC-13).
//!
//! Split from [`crate::report::render`] so it stays under this crate's
//! module-size ceiling.

use crate::paired::{PairedRunObservation, PairedRunOutcome};

/// The paired-run table, one row per plan declaring `paired_run`, or nothing
/// at all when none does — so a report without one renders the bytes it did
/// before.
pub(crate) fn paired_run_table(observations: &[PairedRunObservation]) -> Vec<String> {
    if observations.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        "| Metric | Plan | Status | Differing | Comparable | Required | Detail |".to_owned(),
        "| --- | --- | --- | --- | --- | --- | --- |".to_owned(),
    ];
    for observation in observations {
        let differing = match observation.outcome {
            PairedRunOutcome::Observed { differing } => differing.to_string(),
            _ => "-".to_owned(),
        };
        lines.push(format!(
            "| {} | {} | {} | {differing} | {} | {} | {} |",
            observation.metric,
            observation.plan_id,
            observation.outcome.as_str(),
            observation.comparisons,
            observation.minimum_pairs,
            observation.reasons.join("; ").replace('|', "\\|"),
        ));
    }
    lines
}
