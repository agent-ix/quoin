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

use crate::ids::{SuiteId, TrustDecisionId};

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

/// Where append-only experiment records live (FR-048).
pub const EXPERIMENTS_DIR: &str = "experiments";

/// Where append-only operational evidence records live (FR-048).
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

/// One trust decision, named by its id.
#[must_use]
pub fn trust_decision_path(id: &TrustDecisionId) -> String {
    format!("{TRUST_DIR}/{id}.json")
}

/// The directory holding one suite's records of one kind.
#[must_use]
pub fn suite_dir(family: &str, suite: &SuiteId) -> String {
    format!("{family}/{suite}")
}

/// An assurance record's file, `<stem>.json`.
///
/// The stem is the record id (a UUID); see
/// [`crate::assurance_records::record_file_stem`].
#[must_use]
pub fn assurance_record_path(family: &str, stem: &str) -> String {
    format!("{family}/{stem}.json")
}

/// A record's file stem: its caller-supplied timestamp, with every byte outside
/// ASCII alphanumerics and `-` encoded as `~HH`. The encoding is injective and
/// the stem can never escape the suite directory.
#[must_use]
pub fn record_stem(timestamp: &str) -> String {
    if timestamp.is_empty() {
        return "~E".to_owned();
    }

    let mut stem = String::with_capacity(timestamp.len());
    for byte in timestamp.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' {
            stem.push(char::from(byte));
        } else {
            stem.push('~');
            stem.push(
                char::from_digit(u32::from(byte >> 4), 16)
                    .unwrap_or('0')
                    .to_ascii_uppercase(),
            );
            stem.push(
                char::from_digit(u32::from(byte & 0x0f), 16)
                    .unwrap_or('0')
                    .to_ascii_uppercase(),
            );
        }
    }
    stem
}

/// `<family>/<suite>/<stem>.json` — one file is one record of one suite.
#[must_use]
pub fn record_path(family: &str, suite: &SuiteId, timestamp: &str) -> String {
    format!("{family}/{suite}/{}.json", record_stem(timestamp))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{assurance_record_path, record_path, record_stem, trust_decision_path};
    use crate::ids::{SuiteId, TrustDecisionId};

    #[test]
    fn the_frozen_layout_is_reproduced_exactly() {
        let suite = SuiteId::new("SUITE-1");
        assert_eq!(
            record_path("runs", &suite, "2026-09-27T00:00:00Z"),
            "runs/SUITE-1/2026-09-27T00~3A00~3A00Z.json"
        );
        assert_eq!(record_stem("../x/y"), "~2E~2E~2Fx~2Fy");
        assert_eq!(record_stem(""), "~E");
        assert_ne!(record_stem(""), record_stem("undated"));
        assert_eq!(
            trust_decision_path(&TrustDecisionId::parse("ETD-12").unwrap()),
            "trust/ETD-12.json"
        );
        assert_eq!(
            assurance_record_path("experiments", "0b9a7a52-8f0c-4d0e-9d57-3f1b2c6a1e44"),
            "experiments/0b9a7a52-8f0c-4d0e-9d57-3f1b2c6a1e44.json"
        );
    }

    #[test]
    fn distinct_timestamps_never_alias_a_record_path() {
        let suite = SuiteId::new("SUITE-1");
        assert_ne!(
            record_path("runs", &suite, "2026-09-27T00:00:00Z"),
            record_path("runs", &suite, "2026-09-27T00-00-00Z")
        );
    }
}
