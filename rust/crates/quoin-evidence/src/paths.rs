// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Where every record lives, relative to the store root.
//!
//! **This layout is a frozen compatibility surface.** Every store in the
//! ecosystem is already laid out this way and NFR-025 forbids moving a byte, so
//! these functions reproduce the retained `src/evidence/store.ts` spelling
//! exactly rather than improving on it.
//!
//! The paths are store-root-relative and `/`-separated. The root itself —
//! `<repo>/spec/evidence` — belongs to `quoin-store`
//! ([`quoin_store::store::store_root`]) because change assurance writes its own
//! family beneath the same root; reaching into evidence for it was one half of
//! the import cycle agent-ix/quoin#376 closed.

use crate::ids::{Commit, SuiteId, TrustDecisionId};

/// Where a suite's run records live, relative to the store root.
pub const RUNS_DIR: &str = "runs";

/// Where a suite's finding-shaped scan records live, relative to the store root.
///
/// Separate from [`RUNS_DIR`] because the two answer different questions and a
/// reader must not have to open a file to learn which kind it is.
pub const SCANS_DIR: &str = "scans";

/// Where source-level mock inspection records live.
pub const MOCK_INSPECTIONS_DIR: &str = "mock-inspections";

/// Where use-specific evidence-producer reliance decisions live.
pub const TRUST_DIR: &str = "trust";

/// Where content-addressed experiment records live (FR-048).
pub const EXPERIMENTS_DIR: &str = "experiments";

/// Where content-addressed operational evidence records live (FR-048).
pub const OPERATIONAL_EVIDENCE_DIR: &str = "operational";

/// The authored suite registry.
#[must_use]
pub fn suites_path() -> String {
    "suites.md".to_owned()
}

/// The authored inspections registry.
#[must_use]
pub fn inspections_path() -> String {
    "inspections.md".to_owned()
}

/// The unified binding graph.
#[must_use]
pub fn bindings_path() -> String {
    "bindings.json".to_owned()
}

/// The ratchet baseline.
#[must_use]
pub fn baseline_path() -> String {
    "baseline.json".to_owned()
}

/// One trust decision, named by its id.
#[must_use]
pub fn trust_decision_path(id: &TrustDecisionId) -> String {
    format!("{TRUST_DIR}/{id}.json")
}

/// `runs/<suite>/<commit12>.json` — one file is one run of one suite.
#[must_use]
pub fn run_path(suite: &SuiteId, commit: &Commit) -> String {
    record_path(RUNS_DIR, suite, commit)
}

/// `scans/<suite>/<commit12>.json` — one file is one scan of one suite.
#[must_use]
pub fn scan_path(suite: &SuiteId, commit: &Commit) -> String {
    record_path(SCANS_DIR, suite, commit)
}

/// `mock-inspections/<suite>/<commit12>.json`.
#[must_use]
pub fn mock_inspection_path(suite: &SuiteId, commit: &Commit) -> String {
    record_path(MOCK_INSPECTIONS_DIR, suite, commit)
}

/// The directory holding one suite's records of one kind.
#[must_use]
pub fn suite_dir(family: &str, suite: &SuiteId) -> String {
    format!("{family}/{suite}")
}

/// A content-addressed record's file, `sha256-<64 hex>.json`.
///
/// The hex without its `sha256:` prefix, because a colon is not a portable
/// file-name character and the retained store already made that choice.
#[must_use]
pub fn assurance_record_path(family: &str, hex: &str) -> String {
    format!("{family}/sha256-{hex}.json")
}

fn record_path(family: &str, suite: &SuiteId, commit: &Commit) -> String {
    format!("{family}/{suite}/{}.json", commit.short())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{
        assurance_record_path, mock_inspection_path, run_path, scan_path, trust_decision_path,
    };
    use crate::ids::{Commit, SuiteId, TrustDecisionId};

    #[test]
    fn the_frozen_layout_is_reproduced_exactly() {
        let suite = SuiteId::new("SUITE-1");
        let commit = Commit::new("0123456789abcdef0123");
        assert_eq!(run_path(&suite, &commit), "runs/SUITE-1/0123456789ab.json");
        assert_eq!(
            scan_path(&suite, &commit),
            "scans/SUITE-1/0123456789ab.json"
        );
        assert_eq!(
            mock_inspection_path(&suite, &commit),
            "mock-inspections/SUITE-1/0123456789ab.json"
        );
        assert_eq!(
            trust_decision_path(&TrustDecisionId::parse("ETD-12").unwrap()),
            "trust/ETD-12.json"
        );
        assert_eq!(
            assurance_record_path("experiments", &"a".repeat(64)),
            format!("experiments/sha256-{}.json", "a".repeat(64))
        );
    }
}
